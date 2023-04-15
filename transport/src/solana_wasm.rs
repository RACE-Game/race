//! Solana Transport for WASM
//!
//! This implementation depends on [SolanaWeb3](https://github.com/solana-labs/solana-web3.js).
//! We assume the module is exported as `window.solanaWeb3`.
//!
//! The wasm implementation is for using in `solana-sdk`.
#![cfg(target_arch = "wasm32")]
#![allow(unused)]

use self::types::*;
use crate::error::{TransportError, TransportResult};
use crate::wasm_trait::TransportLocalT;
use crate::wasm_utils::*;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use gloo::console::{debug, error, info, warn};
use js_sys::{Function, Object, Promise, Reflect, Uint8Array};

use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, DepositParams, GameAccount, GameBundle, GameRegistration, JoinParams,
    PlayerJoin, PlayerProfile, RegisterGameParams, RegistrationAccount, ServerAccount, ServerJoin,
    UnregisterGameParams, VoteParams,
};
use race_solana_types::constants::{EMPTY_PUBKEY, PROFILE_ACCOUNT_LEN, PROFILE_SEED, PROGRAM_ID};
use race_solana_types::instruction::RaceInstruction;
use race_solana_types::{
    constants::GAME_ACCOUNT_LEN,
    state::{GameState, PlayerState, RegistryState, ServerState},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub struct SolanaWasmTransport {
    program_id: Pubkey,
    conn: Connection,
}

mod types;

#[async_trait(?Send)]
#[allow(unused)]
impl TransportLocalT for SolanaWasmTransport {
    async fn create_game_account(
        &self,
        wallet: &JsValue,
        params: CreateGameAccountParams,
    ) -> TransportResult<String> {
        let CreateGameAccountParams {
            title,
            bundle_addr,
            token_addr,
            max_players,
            data,
            min_deposit,
            max_deposit,
        } = params;
        let wallet_pubkey = Self::wallet_pubkey(wallet);
        let bundle_pubkey = Pubkey::try_new(&bundle_addr)?;
        let game_account = Keypair::new();
        let game_account_pubkey = game_account.public_key();
        let tx = Transaction::new(&self.conn, &wallet_pubkey);
        let lamports = self
            .conn
            .get_minimum_balance_for_rent_exemption(GAME_ACCOUNT_LEN)
            .await;
        let create_game_account_ix = Instruction::create_account(
            &game_account_pubkey,
            &game_account_pubkey,
            lamports,
            GAME_ACCOUNT_LEN,
            &self.program_id,
        );
        tx.add(&create_game_account_ix);

        let token_pubkey = Pubkey::try_new(&token_addr)?;
        let temp_stake_account = Keypair::new();
        let temp_stake_account_pubkey = temp_stake_account.public_key();
        let temp_stake_account_lamports = self
            .conn
            .get_minimum_balance_for_rent_exempt_account()
            .await;
        let create_temp_stake_account_ix = Instruction::create_account(
            &wallet_pubkey,
            &temp_stake_account_pubkey,
            temp_stake_account_lamports,
            Account::len() as _,
            &spl_token_program_id(),
        );
        tx.add(&create_temp_stake_account_ix);

        let init_temp_stake_account_ix = Instruction::create_initialize_account_instruction(
            &temp_stake_account_pubkey,
            &token_pubkey,
            &wallet_pubkey,
        );
        tx.add(&init_temp_stake_account_ix);

        let create_game_ix = Instruction::new_with_borsh(
            &self.program_id,
            &RaceInstruction::CreateGameAccount {
                params: race_solana_types::types::CreateGameAccountParams {
                    title,
                    max_players,
                    data,
                    min_deposit,
                    max_deposit,
                },
            },
            vec![
                AccountMeta::new_readonly(&wallet_pubkey, true),
                AccountMeta::new(&game_account_pubkey, false),
                AccountMeta::new(&temp_stake_account_pubkey, true),
                AccountMeta::new_readonly(&token_pubkey, false),
                AccountMeta::new_readonly(&spl_token_program_id(), false),
                AccountMeta::new_readonly(&bundle_pubkey, false),
            ],
        );
        tx.add(&create_game_ix);

        self.conn.send_transaction_and_confirm(wallet, &tx).await;

        Ok(game_account_pubkey.to_base58())
    }

    async fn close_game_account(
        &self,
        wallet: &JsValue,
        params: CloseGameAccountParams,
    ) -> TransportResult<()> {
        todo!()
    }

    async fn join(&self, wallet: &JsValue, params: JoinParams) -> TransportResult<()> {
        let JoinParams {
            player_addr,
            game_addr,
            amount,
            access_version,
            position,
        } = params;
        let wallet_pubkey = Self::wallet_pubkey(wallet);
        let player_profile_pubkey =
            Pubkey::create_with_seed(&wallet_pubkey, PROFILE_SEED, &self.program_id).await;
        let game_account_pubkey = Pubkey::try_new(&game_addr)?;
        let game_state: GameState = self
            .conn
            .get_account_state(&game_account_pubkey)
            .await
            .unwrap();

        let mint_pubkey = Pubkey::try_new(&game_state.token_mint.to_string())?;
        let stake_account_pubkey = Pubkey::try_new(&game_state.stake_account.to_string())?;
        let is_wsol = mint_pubkey.eq(&spl_native_mint());
        let (pda, _) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_buffer()], &self.program_id);

        let tx = Transaction::new(&self.conn, &wallet_pubkey);

        let temp_account = Keypair::new();
        let temp_account_pubkey = temp_account.public_key();
        let temp_account_lamports = self
            .conn
            .get_minimum_balance_for_rent_exempt_account()
            .await;

        let create_temp_account_ix = Instruction::create_account(
            &wallet_pubkey,
            &temp_account_pubkey,
            temp_account_lamports,
            Account::len(),
            &spl_token_program_id(),
        );
        tx.add(&create_temp_account_ix);

        let sync_native_ix = Instruction::create_sync_native_instruction(&temp_account_pubkey);
        if is_wsol {
            let amount = amount - temp_account_lamports as u64;
            let transfer_sol_ix =
                Instruction::transfer(&wallet_pubkey, &temp_account_pubkey, amount);
            tx.add(&transfer_sol_ix);
        } else {
            let init_temp_account_ix = Instruction::create_initialize_account_instruction(
                &temp_account_pubkey,
                &mint_pubkey,
                &wallet_pubkey,
            );
            tx.add(&init_temp_account_ix);
        }

        let join_ix = Instruction::new_with_borsh(
            &self.program_id,
            &RaceInstruction::JoinGame {
                params: race_solana_types::types::JoinParams {
                    amount,
                    access_version,
                    position,
                },
            },
            vec![
                AccountMeta::new_readonly(&wallet_pubkey, true),
                AccountMeta::new(&temp_account_pubkey, false),
                AccountMeta::new(&game_account_pubkey, false),
                AccountMeta::new_readonly(&mint_pubkey, false),
                AccountMeta::new(&stake_account_pubkey, false),
                AccountMeta::new(&pda, false),
                AccountMeta::new_readonly(&spl_token_program_id(), false),
            ],
        );
        tx.add(&join_ix);
        self.conn.send_transaction_and_confirm(&wallet, &tx).await;
        debug!("Transaction confirmed");
        Ok(())
    }

    async fn deposit(&self, wallet: &JsValue, params: DepositParams) -> TransportResult<()> {
        todo!()
    }

    async fn vote(&self, wallet: &JsValue, params: VoteParams) -> TransportResult<()> {
        todo!()
    }

    async fn create_player_profile(
        &self,
        wallet: &JsValue,
        params: CreatePlayerProfileParams,
    ) -> TransportResult<String> {
        debug!("Create profile, wallet:", wallet);
        let wallet_pubkey = Self::wallet_pubkey(wallet);
        debug!("Wallet pubkey:", wallet_pubkey.to_base58());
        let CreatePlayerProfileParams { addr, nick, pfp } = params;
        debug!(format!("Nick: {} , Pfp: {:?}", nick, pfp));
        let profile_account_pubkey =
            Pubkey::create_with_seed(&wallet_pubkey, PROFILE_SEED, &self.program_id).await;
        debug!(
            "Profile account pubkey:",
            profile_account_pubkey.to_base58()
        );
        let pfp_pubkey = pfp
            .and_then(|pfp| Pubkey::try_new(&pfp).ok())
            .unwrap_or_else(|| Pubkey::try_new(EMPTY_PUBKEY).unwrap());

        let lamports = self
            .conn
            .get_minimum_balance_for_rent_exemption(PROFILE_ACCOUNT_LEN)
            .await;
        let tx = Transaction::new(&self.conn, &wallet_pubkey);

        // Only create account when profile doesn't exist
        if self
            .conn
            .get_account_data(&profile_account_pubkey)
            .await
            .is_none()
        {
            debug!("Create profile account, spend lamports:", lamports);
            let create_account_ix = Instruction::create_account_with_seed(
                &wallet_pubkey,
                &profile_account_pubkey,
                &wallet_pubkey,
                PROFILE_SEED,
                lamports,
                PROFILE_ACCOUNT_LEN,
                &self.program_id,
            );
            tx.add(&create_account_ix);
        }

        let init_profile_ix = Instruction::new_with_borsh(
            &self.program_id,
            &RaceInstruction::CreatePlayerProfile {
                params: race_solana_types::types::CreatePlayerProfileParams { nick },
            },
            vec![
                AccountMeta::new_readonly(&wallet_pubkey, true),
                AccountMeta::new(&profile_account_pubkey, false),
                AccountMeta::new_readonly(&pfp_pubkey, false),
            ],
        );
        tx.add(&init_profile_ix);
        self.conn.send_transaction_and_confirm(&wallet, &tx).await;
        debug!("Transaction confirmed");
        Ok(profile_account_pubkey.to_base58())
    }

    async fn publish_game(&self, wallet: &JsValue, bundle: GameBundle) -> TransportResult<String> {
        todo!()
    }

    async fn create_registration(
        &self,
        wallet: &JsValue,
        params: CreateRegistrationParams,
    ) -> TransportResult<String> {
        unimplemented!()
    }

    async fn register_game(
        &self,
        wallet: &JsValue,
        params: RegisterGameParams,
    ) -> TransportResult<()> {
        unimplemented!()
    }

    async fn unregister_game(
        &self,
        wallet: &JsValue,
        params: UnregisterGameParams,
    ) -> TransportResult<()> {
        unimplemented!()
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let pubkey = Pubkey::try_new(addr).ok()?;
        debug!(format!("Get game account at {}", addr));
        let state: GameState = self.conn.get_account_state(&pubkey).await?;
        let bundle_addr = state.bundle_addr.to_string();
        let transactor_addr = match state.transactor_addr {
            Some(pubkey) => Some(pubkey.to_string()),
            None => None,
        };
        Some(GameAccount {
            addr: addr.to_owned(),
            title: state.title,
            settle_version: state.settle_version,
            bundle_addr,
            access_version: state.access_version,
            players: state
                .players
                .into_iter()
                .map(|p| PlayerJoin {
                    addr: p.addr.to_string(),
                    position: p.position as usize,
                    balance: p.balance,
                    access_version: p.access_version,
                })
                .collect(),
            servers: state
                .servers
                .into_iter()
                .map(|s| ServerJoin {
                    addr: s.addr.to_string(),
                    endpoint: s.endpoint,
                    access_version: s.access_version,
                })
                .collect(),
            transactor_addr,
            max_players: state.max_players,
            data_len: state.data_len,
            data: *state.data,
            // TODO: impl the following fields
            deposits: Vec::new(),
            votes: Vec::new(),
            unlock_time: None,
        })
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        let pubkey = Pubkey::try_new(addr).ok()?;
        let state: PlayerState = self.conn.get_account_state(&pubkey).await?;
        Some(PlayerProfile {
            addr: addr.to_owned(),
            nick: state.nick,
            pfp: state.pfp.map(|p| p.to_string()),
        })
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        let pubkey = Pubkey::try_new(addr).ok()?;
        debug!(format!("Get server profile at {}", addr));
        let state: ServerState = self.conn.get_account_state(&pubkey).await?;
        Some(ServerAccount {
            addr: addr.to_owned(),
            owner_addr: state.owner.to_string(),
            endpoint: state.endpoint,
        })
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let pubkey = Pubkey::try_new(addr).ok()?;
        debug!(format!("Get registration at {}", addr));
        let state: RegistryState = self.conn.get_account_state(&pubkey).await?;
        let games: Vec<GameRegistration> = state
            .games
            .into_iter()
            .map(|r| GameRegistration {
                title: r.title,
                addr: r.addr.to_string(),
                reg_time: r.reg_time,
                bundle_addr: r.bundle_addr.to_string(),
            })
            .collect();
        debug!(format!("Found {} games at {}", games.len(), addr));
        Some(RegistrationAccount {
            addr: addr.to_owned(),
            is_private: state.is_private,
            size: state.size,
            owner: Some(state.owner.to_string()),
            games,
        })
    }
}

impl SolanaWasmTransport {
    pub fn try_new(rpc: String) -> TransportResult<Self> {
        let conn = Connection::new(&rpc);
        let program_id = Pubkey::try_new(PROGRAM_ID).unwrap();
        Ok(Self { conn, program_id })
    }

    fn wallet_pubkey(wallet: &JsValue) -> Pubkey {
        if wallet.is_falsy() {
            panic!("Wallet is not connected");
        }
        let value = rget(wallet, "publicKey");
        Pubkey { value }
    }
}

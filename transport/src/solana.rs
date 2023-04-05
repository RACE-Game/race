#![cfg(not(target_arch = "wasm32"))]
#![allow(unused_variables, unused_imports)]
use crate::error::{TransportError, TransportResult};
use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use jsonrpsee::core::Error;
use race_core::{
    error::Result,
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, GameRegistration,
        JoinParams, PlayerJoin, PlayerProfile, RegisterGameParams, RegisterServerParams,
        RegistrationAccount, ServeParams, ServerAccount, ServerJoin, SettleOp, SettleParams,
        UnregisterGameParams, VoteParams,
    },
};
use race_solana_types::constants::{
    GAME_ACCOUNT_LEN, MAX_SERVER_NUM, NAME_LEN, PROFILE_ACCOUNT_LEN, PROFILE_SEED, PROGRAM_ID,
    REGISTRY_ACCOUNT_LEN, SERVER_ACCOUNT_LEN, SOL,
};
use race_solana_types::instruction::RaceInstruction;
use race_solana_types::state::{self, GameReg, GameState, PlayerState, RegistryState, ServerState};
use race_solana_types::types as solana_types;

use serde_json;
use std::{fs::{read_to_string, File}, borrow::BorrowMut};
use std::path::PathBuf;
use std::str::FromStr;

use solana_client::{
    rpc_client::{RpcClient, RpcClientConfig},
    rpc_config::RpcSendTransactionConfig,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::{create_account, create_account_with_seed};
use solana_sdk::transaction::Transaction;
use solana_sdk::{commitment_config::CommitmentConfig, program_pack::Pack};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::ParsePubkeyError,
    signature::Signature,
};
use solana_sdk::{message::Message, system_program};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    check_id, check_program_account, id, instruction::initialize_account, state::Account, ID,
};

fn read_keypair(path: PathBuf) -> TransportResult<Keypair> {
    let keypair = solana_sdk::signature::read_keypair_file(path)
        .map_err(|e| TransportError::InvalidKeyfile(e.to_string()))?;
    Ok(keypair)
}

pub struct SolanaTransport {
    program_id: Pubkey,
    client: RpcClient,
    keypair: Keypair,
}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for SolanaTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        // TODO: Discuss title allowed len
        if params.title.len() > 30 {
                // FIXME: Use TransportError
                return Err(race_core::error::Error::Custom(
                    "Game title exceeds 30 chars".to_string(),
                ));
        }

        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let bundle_pubkey = Pubkey::from_str(&params.bundle_addr)
            .map_err(|_| TransportError::InvalidBundleAddress)?;
        let game_account = Keypair::new();
        let game_account_pubkey = game_account.pubkey();
        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(GAME_ACCOUNT_LEN)
            .map_err(|_| TransportError::NoEnoughLamports)?;
        let create_game_account_ix = create_account(
            &payer_pubkey,
            &game_account_pubkey,
            lamports,
            GAME_ACCOUNT_LEN as u64,
            &self.program_id,
        );

        let token_pubkey = Pubkey::from_str(SOL).unwrap();
        let stake_account = Keypair::new();
        let stake_account_pubkey = stake_account.pubkey();
        let stake_account_len = Account::LEN;
        let stake_lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(stake_account_len)
            .map_err(|_| TransportError::NoEnoughLamports)?;
        let create_stake_account_ix = create_account(
            &payer_pubkey,
            &stake_account_pubkey,
            stake_lamports,
            stake_account_len as u64,
            &ID,
        );

        let init_stake_account_ix =
            initialize_account(&ID, &stake_account_pubkey, &token_pubkey, &payer_pubkey)
                .map_err(|_| TransportError::InitInstructionFailed)?;

        // FIXME: limit title to 16 or 30 chars

        let create_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CreateGameAccount {
                params: solana_types::CreateGameAccountParams {
                    title: params.title,
                    max_players: params.max_players,
                    data: params.data,
                },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, true),
                AccountMeta::new_readonly(token_pubkey, false),
                AccountMeta::new_readonly(ID, false),
                AccountMeta::new_readonly(bundle_pubkey, false),
                // TODO: add scene pubkey
            ],
        );
        let message = Message::new(
            &[
                create_game_account_ix,
                create_stake_account_ix,
                init_stake_account_ix,
                create_game_ix,
            ],
            Some(&payer.pubkey())
        );
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer, &game_account, &stake_account], blockhash);
        self.send_transaction(tx)?;
        Ok(game_account_pubkey.to_string())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        // payer is initializer/owner of the to-be-closed game
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let game_account_pubkey = Pubkey::from_str(&params.addr)
            .map_err(|_| TransportError::InvalidPubkey(params.addr))?;
        let game_account_data = &self
            .client
            .get_account_data(&game_account_pubkey)
            .map_err(|_| TransportError::InvalidPubkey(game_account_pubkey.to_string()))?;
        let game_state = GameState::try_from_slice(&game_account_data)?;
        let stake_account_pubkey = game_state.stake_addr.clone();

        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &self.program_id);

        let close_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CloseGameAccount,
            vec![
                AccountMeta::new(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, false),
                AccountMeta::new_readonly(pda, false),
                AccountMeta::new_readonly(ID, false),
            ],
        );

        let message = Message::new(&[close_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);

        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer], blockhash);

        self.send_transaction(tx)?;
        Ok(())
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<String> {
        // Check endpoint URL len
        if params.endpoint.len() > 50 {
            // FIXME: Use TransportError
            return Err(race_core::error::Error::Custom(
                "Endpoint length exceeds 50 chars".to_string(),
            ));
        }
        // Create server profile on chain (like creation of a player profile)
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let server_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::PubkeyCreationFailed)?;

        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(SERVER_ACCOUNT_LEN)
            .map_err(|_| TransportError::NoEnoughLamports)?;

        match self.client.get_account(&server_account_pubkey) {
            Ok(_) => {
                // FIXME: Use TransportError
                return Err(race_core::error::Error::Custom(
                    "Server already exists".to_string(),
                ));
            }
            Err(_) => {}
        }

        let create_server_account_ix = create_account_with_seed(
            &payer_pubkey,
            &server_account_pubkey,
            &payer_pubkey,
            PROFILE_SEED,
            lamports,
            SERVER_ACCOUNT_LEN as u64,
            &self.program_id,
        );

        let init_account_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::RegisterServer {
                params: solana_types::RegisterServerParams {
                    endpoint: params.endpoint,
                },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(server_account_pubkey, false),
            ],
        );

        let message = Message::new(
            &[create_server_account_ix, init_account_ix],
            Some(&payer_pubkey),
        );

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;

        tx.sign(&[payer], blockhash);

        self.send_transaction(tx)?;
        Ok(server_account_pubkey.to_string())
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        // Player Join
        // 1. check max player (must be <= max player)
        // 2. check player position 0 to max_ply - 1
        // 3. access_version == current access_version; if different, fail
        // each
        todo!()
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let server_account_pubkey = Self::parse_pubkey(&params.server_addr)?;

        let game_state = self.internal_get_game_account(&game_account_pubkey).await?;

        // Check max server num
        if game_state.servers.len() == MAX_SERVER_NUM {
            return Err(race_core::error::Error::Custom(
                "Server number exceeds the limit".to_string(),
            ));
        }

        let serve_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::ServeGame,
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new_readonly(server_account_pubkey, false),
            ],
        );

        let message = Message::new(&[serve_game_ix], Some(&payer_pubkey));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<String> {
        // Check if nick name exceeds 16 chars
        if params.nick.len() > NAME_LEN {
            // FIXME: Transfer transport error to race error
            return Err(race_core::error::Error::Custom(
                "Nick name should not exceed 16 characters".to_string(),
            ));
        }
        let player = &self.keypair;
        let player_pubkey = player.pubkey();
        let balance = self
            .client
            .get_balance(&player_pubkey)
            .map_err(|_| TransportError::InvalidBalance(player_pubkey.to_string()))?;

        let profile_account_pubkey =
            Pubkey::create_with_seed(&player_pubkey, PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::PubkeyCreationFailed)?;

        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(PROFILE_ACCOUNT_LEN)
            .map_err(|_| TransportError::NoEnoughLamports)?;

        // Check if account already exists
        match self.client.get_account(&profile_account_pubkey) {
            Ok(_) => {
                // FIXME: Use TransportError
                return Err(race_core::error::Error::Custom(
                    "Profile already exists".to_string(),
                ));
            }
            Err(_) => {}
        }

        let create_profile_account_ix = create_account_with_seed(
            &player_pubkey,
            &profile_account_pubkey,
            &player_pubkey,
            PROFILE_SEED,
            lamports,
            PROFILE_ACCOUNT_LEN as u64,
            &self.program_id,
        );

        // TODO: Add Racetoken ATA

        let pfp_pubkey = if params.pfp.is_some() {
            let addr = params.pfp.unwrap();
            Pubkey::from_str(&addr).map_err(|_| TransportError::InvalidPubkey(addr.to_string()))?
        } else {
            let addr = "11111111111111111111111111111111";
            Pubkey::from_str(&addr).map_err(|_| TransportError::InvalidPubkey(addr.to_string()))?
        };

        let init_profile_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CreatePlayerProfile {
                params: solana_types::CreatePlayerProfileParams { nick: params.nick },
            },
            vec![
                AccountMeta::new_readonly(player_pubkey, true),
                AccountMeta::new(profile_account_pubkey, false),
                AccountMeta::new_readonly(pfp_pubkey, false),
            ],
        );

        let message = Message::new(
            &[create_profile_account_ix, init_profile_ix],
            Some(&player_pubkey),
        );

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;

        tx.sign(&[player], blockhash);

        self.send_transaction(tx)?;

        Ok(profile_account_pubkey.to_string())
    }

    // TODO: add close_player_profile

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        // Publish game bundle (similar to minting NFTs)
        todo!()
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        let SettleParams { addr, mut settles } = params;
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let game_account_pubkey = Self::parse_pubkey(&addr)?;
        let game_state = self.internal_get_game_account(&game_account_pubkey).await?;
        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &self.program_id);

        // The settles are required to be in correct order: add < sub < eject.
        // And the sum of settles must be zero.
        settles.sort_by_key(|s| match s.op {
            SettleOp::Eject => 2,
            SettleOp::Add(_) => 0,
            SettleOp::Sub(_) => 1,
        });

        let settles: TransportResult<Vec<race_solana_types::types::Settle>> = settles
            .into_iter()
            .map(|s| {
                Ok(match s.op {
                    SettleOp::Eject => race_solana_types::types::Settle {
                        addr: Pubkey::from_str(&s.addr)?,
                        op: race_solana_types::types::SettleOp::Eject,
                    },
                    SettleOp::Add(amt) => race_solana_types::types::Settle {
                        addr: Pubkey::from_str(&s.addr)?,
                        op: race_solana_types::types::SettleOp::Add(amt),
                    },
                    SettleOp::Sub(amt) => race_solana_types::types::Settle {
                        addr: Pubkey::from_str(&s.addr)?,
                        op: race_solana_types::types::SettleOp::Sub(amt),
                    },
                })
            })
            .collect();
        let settles = settles?;

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(Pubkey::from_str(&addr).unwrap(), false),
            AccountMeta::new_readonly(game_state.token_addr.clone(), false),
            AccountMeta::new_readonly(pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let settle_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::Settle {
                params: race_solana_types::types::SettleParams { settles },
            },
            accounts,
        );

        let message = Message::new(&[settle_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let registry_account = Keypair::new();
        let registry_account_pubkey = registry_account.pubkey();
        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(REGISTRY_ACCOUNT_LEN)
            .map_err(|_| TransportError::NoEnoughLamports)
            .unwrap();
        let create_account_ix = create_account(
            &payer_pubkey,
            &registry_account_pubkey,
            lamports,
            REGISTRY_ACCOUNT_LEN as u64,
            &self.program_id,
        );
        let create_registry_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CreateRegistry {
                params: race_solana_types::types::CreateRegistrationParams {
                    is_private: params.is_private,
                    size: params.size,
                },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(registry_account_pubkey, false),
            ],
        );

        let message = Message::new(
            &[create_account_ix, create_registry_ix],
            Some(&payer.pubkey()),
        );
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)
            .unwrap();
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&payer, &registry_account], blockhash);
        self.send_transaction(tx)?;
        let addr = registry_account_pubkey.to_string();
        Ok(addr)
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let reg_account_pubkey = Self::parse_pubkey(&params.reg_addr)?;
        let reg_state = self.get_registry_state(&reg_account_pubkey).await?;
        println!("payer pubkey {:?}", payer_pubkey);
        println!("game pubkey {:?}", game_account_pubkey);
        println!("reg pubkey {:?}", reg_account_pubkey);
        println!("reg_state addr {:?}", reg_state.addr);
        println!("reg_state owner {:?}", reg_state.owner);

        if reg_state.games.len() == reg_state.size as usize {
                // FIXME: Use TransportError
                return Err(race_core::error::Error::Custom(
                    "Registry already full".to_string(),
                ));
        }

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(reg_account_pubkey, false),
            AccountMeta::new_readonly(game_account_pubkey, false),
        ];

        let register_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::RegisterGame, // TODO: add is_hidden
            accounts
        );

        let message = Message::new(&[register_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;
        println!("5");

        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let reg_account_pubkey = Self::parse_pubkey(&params.reg_addr)?;
        // let reg_state = self.get_registry_state(&reg_account_pubkey).await?;

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(reg_account_pubkey, false),
            AccountMeta::new_readonly(game_account_pubkey, false),
        ];

        let unregister_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            // TODO: add is_hidden param?
            &RaceInstruction::UnregisterGame,
            accounts
        );

        let message = Message::new(&[unregister_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let game_account_pubkey = Self::parse_pubkey(addr).ok()?;
        let state = self
            .internal_get_game_account(&game_account_pubkey)
            .await
            .ok()?;
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
        let game_bundle_pubkey = Self::parse_pubkey(addr).ok()?;
        let bundle_data = self.client.get_account_data(&game_bundle_pubkey).ok()?;
        let bundle_state = GameBundle::try_from_slice(&bundle_data).ok()?;
        let addr = bundle_state.addr.to_string();
        let data = "ARWEAVE BASE64 ADDRESS".to_string();
        Some(GameBundle { addr, data })
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        let profile_pubkey = Self::parse_pubkey(addr).ok()?;
        let profile_data = self.client.get_account_data(&profile_pubkey).ok()?;
        let profile_state = PlayerState::try_from_slice(&profile_data).ok()?;
        let addr = profile_state.addr.to_string();
        let pfp = profile_state.pfp.map(|x| x.to_string());
        Some(PlayerProfile {
            addr,
            nick: profile_state.nick,
            pfp,
        })
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        let server_account_pubkey = Self::parse_pubkey(addr).ok()?;
        let server_account_data = self.client.get_account_data(&server_account_pubkey).ok()?;
        let server_state = ServerState::try_from_slice(&server_account_data).ok()?;
        Some(ServerAccount {
            addr: server_state.addr.to_string(),
            owner_addr: server_state.owner.to_string(),
            endpoint: server_state.endpoint,
        })
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let registry_account_pubkey = Self::parse_pubkey(addr).ok()?;
        let data = self.client.get_account_data(&registry_account_pubkey).ok()?;
        let state = RegistryState::try_from_slice(&data).ok()?;
        Some(RegistrationAccount {
            addr: addr.to_owned(),
            is_private: state.is_private,
            size: state.size,
            owner: Some(state.owner.to_string()),
            games: state
                .games
                .into_iter()
                .map(|g| GameRegistration {
                    title: g.title.clone(),
                    addr: g.addr.to_string(),
                    reg_time: g.reg_time,
                    bundle_addr: "".into(),
                })
                .collect(),
        })
    }
}

impl SolanaTransport {
    pub fn try_new(rpc: String, keyfile: PathBuf) -> TransportResult<Self> {
        let program_id = Pubkey::from_str(PROGRAM_ID)?;
        SolanaTransport::try_new_with_program_id(rpc, keyfile, program_id)
    }

    pub(crate) fn try_new_with_program_id(
        rpc: String,
        keyfile: PathBuf,
        program_id: Pubkey,
    ) -> TransportResult<Self> {
        println!(
            "Create Solana transport: RPC: {}, program_id: {:?}",
            rpc, program_id
        );
        let commitment = if cfg!(test) {
            CommitmentConfig::confirmed()
        } else {
            CommitmentConfig::finalized()
        };
        let client = RpcClient::new_with_commitment(rpc, commitment);
        let keypair = read_keypair(keyfile)?;
        Ok(Self {
            client,
            keypair,
            program_id,
        })
    }

    fn parse_pubkey(addr: &str) -> TransportResult<Pubkey> {
        Pubkey::from_str(addr).map_err(|_| TransportError::InvalidConfig)
    }

    fn get_blockhash(&self) -> TransportResult<Hash> {
        self.client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)
    }

    fn send_transaction(&self, tx: Transaction) -> TransportResult<Signature> {
        let skip_preflight = if cfg!(test) { true } else { false };

        // let sig = self
        //     .client
        //     .send_and_confirm_transaction(&tx)
        //     .map_err(|e| TransportError::ClientSendTransactionFailed(e.to_string()))?;

        let sig = self
            .client
            .send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    skip_preflight,
                    ..RpcSendTransactionConfig::default()
                },
            )
            .map_err(|e| TransportError::ClientSendTransactionFailed(e.to_string()))?;

        Ok(sig)
    }

    /// Get the state of an on-chain game account by its public key.
    /// Not for public API usage
    async fn internal_get_game_account(
        &self,
        game_account_pubkey: &Pubkey,
    ) -> TransportResult<GameState> {
        let data = self
            .client
            .get_account_data(&game_account_pubkey)
            .or(Err(TransportError::GameAccountNotFound))?;

        GameState::try_from_slice(&data).or(Err(TransportError::GameStateDeserializeError))
    }

    /// Get the state of an on-chain server account
    /// Not for public API usage
    #[allow(unused)]
    async fn get_server_state(
        &self,
        server_account_pubkey: &Pubkey,
    ) -> TransportResult<ServerState> {
        let data = self
            .client
            .get_account_data(&server_account_pubkey)
            .or(Err(TransportError::ServerAccountDataNotFound))?;

        ServerState::try_from_slice(&data).or(Err(TransportError::ServerStateDeserializeError))
    }

    /// Get the state of an on-chain registry account
    /// Not for public API usage
    async fn get_registry_state(
        &self,
        registry_account_pubkey: &Pubkey,
    ) -> TransportResult<RegistryState> {
        let data = self
            .client
            .get_account_data(&registry_account_pubkey)
            .or(Err(TransportError::RegistryAccountDataNotFound))?;

        RegistryState::try_from_slice(&data).or(Err(TransportError::RegistryStateDeserializeError))
    }
}

impl From<ParsePubkeyError> for TransportError {
    fn from(value: ParsePubkeyError) -> Self {
        TransportError::ParseAddressError
    }
}

#[cfg(test)]
mod tests {
    use solana_client::rpc_config::RpcProgramAccountsConfig;

    use super::*;

    fn read_program_id() -> anyhow::Result<Pubkey> {
        let proj_root = project_root::get_project_root()?;
        let keyfile_path = proj_root.join("target/deploy/race_solana-keypair.json".to_string());
        let program_keypair = read_keypair(keyfile_path)?;
        let program_id = program_keypair.pubkey();
        println!("program id: {}", program_id);
        Ok(program_id)
    }

    #[test]
    fn test_read_program_id() -> anyhow::Result<()> {
        read_program_id()?;
        Ok(())
    }

    #[test]
    fn test_read_keypair() -> anyhow::Result<()> {
        let keypair = read_keypair(
            shellexpand::tilde("~/.config/solana/id.json")
                .to_string()
                .into(),
        )?;
        Ok(())
    }

    fn get_transport() -> anyhow::Result<SolanaTransport> {
        let transport = SolanaTransport::try_new_with_program_id(
            "http://localhost:8899".into(),
            shellexpand::tilde("~/.config/solana/id.json")
                .to_string()
                .into(),
            read_program_id()?,
        )?;
        Ok(transport)
    }

    #[test]
    fn test_get_transport() -> anyhow::Result<()> {
        get_transport()?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_registry() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = transport
            .create_registration(CreateRegistrationParams {
                is_private: false,
                size: 100,
            })
            .await?;
        println!("Created registry at {}", addr);
        // let addr = "Dk4bP7t8hBq5BuhUbEVkuo8yongCVRQ1ySXDoDLPJZgW".to_string();

        // if addr.len() > 0 {
        //     let reg = transport.get_registration(&addr).await.unwrap();
        //     assert_eq!(reg.addr, addr);
        //     assert_eq!(reg.is_private, false);
        //     assert_eq!(reg.size, 100);
        // }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = transport
            .create_game_account(CreateGameAccountParams {
                title: "HHHHH".to_string(),
                bundle_addr: "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_owned(),
                max_players: 9,
                data: Vec::<u8>::new(),
            })
            .await?;
        println!("Created game at {}", addr);

        if addr.len() > 0 {
            let game = transport.get_game_account(&addr).await.unwrap();
            assert_eq!(game.addr, addr);
            assert_eq!(
                game.bundle_addr,
                "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_string()
            );
            assert_eq!(game.title, "HHHHH".to_string());
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_close_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = transport
            .create_game_account(CreateGameAccountParams {
                title: "HHHHH".to_string(),
                bundle_addr: "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_owned(),
                max_players: 9,
                data: Vec::<u8>::new(),
            })
            .await?;

        println!("To close game account {}", addr);
        transport
            .close_game_account(CloseGameAccountParams { addr })
            .await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = transport
            .register_game(RegisterGameParams {
                game_addr: "7eQZSoKurnDhNQ8brPuaP9rb9r8JDc19cqyajKzkXJDq".to_string(),
                reg_addr: "HsD2b3RadjhwUDPz3AcBqfspq1y8B85SLtQh7cXZmqwX".to_string()
            })
            .await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_player_profile() -> anyhow::Result<()> {
        let transport = get_transport()?;
        // Create a player profile
        let profile_addr = transport
            .create_player_profile(CreatePlayerProfileParams {
                addr: "HHHHHJJJJKKKKLLLLPPPPOOOOIIIIUUUU".to_string(),
                nick: "Jackson".to_owned(),
                pfp: None,
            })
            .await?;

        println!("Created profile is {}", profile_addr);

        // Try to get it
        // let profile_addr: &str = "FEZ6ki7Jy1fG4sYLwEiDZQm1bk5H7v4JpLc3EHKs355K";
        let profile = transport.get_player_profile(&profile_addr).await.unwrap();
        assert_eq!("Jackson".to_string(), profile.nick);
        assert_eq!(None, profile.pfp);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_server_account() -> anyhow::Result<()> {
        let transport = get_transport().unwrap();
        // Create a player profile
        let server_addr = transport
            .register_server(RegisterServerParams {
                endpoint: "https://api.testnet.solana.com".to_string(),
            })
            .await?;

        println!("Created profile is {}", server_addr);

        // Try to get it
        // let addr = "8BUgJXM54YbiLSFf9pYjUejSej39G8U9VeAimQdmd43u";
        let server_state = transport.get_server_account(&server_addr).await.unwrap();
        assert_eq!(
            "https://api.testnet.solana.com".to_string(),
            server_state.endpoint
        );

        Ok(())
    }

    async fn test_settle() -> anyhow::Result<()> {
        Ok(())
    }
}

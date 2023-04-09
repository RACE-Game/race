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
use crate::wasm_utils::*;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use gloo::console::{debug, error, info, warn};
use js_sys::{Function, Object, Promise, Reflect, Uint8Array};

use race_core::{
    error::Result,
    transport::TransportLocalT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, GameRegistration,
        JoinParams, PlayerJoin, PlayerProfile, RegisterGameParams, RegistrationAccount,
        ServerAccount, ServerJoin, UnregisterGameParams, VoteParams,
    },
};
use race_solana_types::{
    constants::GAME_ACCOUNT_LEN,
    state::{GameState, PlayerState, RegistryState, ServerState},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub struct SolanaWasmTransport {
    conn: Connection,
}

mod types;

#[async_trait(?Send)]
#[allow(unused)]
impl TransportLocalT for SolanaWasmTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        let bundle_pubkey = Pubkey::new(&params.bundle_addr);
        let game_account = Keypair::new();
        let game_account_pubkey = game_account.public_key();
        let lamports = self.conn.get_minimum_balance_for_rent_exemption(GAME_ACCOUNT_LEN);
        let create_game_ix = Instruction::create_account(
            &game_account_pubkey,
            &game_account_pubkey,
            lamports,
            GAME_ACCOUNT_LEN,
        );
        let tx = Transaction::new(&self.conn, &game_account_pubkey);
        tx.add(&create_game_ix);
        Ok(game_account_pubkey.to_base58())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        todo!()
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        todo!()
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<String> {
        todo!()
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        todo!()
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        todo!()
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        todo!()
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        todo!()
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let pubkey = Pubkey::new(addr);
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
        let pubkey = Pubkey::new(addr);
        debug!(format!("Get player profile at {}", addr));
        let state: PlayerState = self.conn.get_account_state(&pubkey).await?;
        Some(PlayerProfile {
            addr: addr.to_owned(),
            nick: state.nick,
            pfp: state.pfp.map(|p| p.to_string()),
        })
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        let pubkey = Pubkey::new(addr);
        debug!(format!("Get server profile at {}", addr));
        let state: ServerState = self.conn.get_account_state(&pubkey).await?;
        Some(ServerAccount {
            addr: addr.to_owned(),
            owner_addr: state.owner.to_string(),
            endpoint: state.endpoint,
        })
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let pubkey = Pubkey::new(addr);
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
        Ok(Self { conn })
    }
}

//! Solana Transport for WASM
//!
//! This implementation depends on [SolanaWeb3](https://github.com/solana-labs/solana-web3.js).
//! We assume the module is exported as `window.solanaWeb3`.
//!
//! The wasm implementation is for using in `solana-sdk`.
#![cfg(target_arch = "wasm32")]
#![allow(unused)]

use crate::error::{TransportError, TransportResult};
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
use race_solana_types::state::{GameState, PlayerState, RegistryState, ServerState};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub struct SolanaWasmTransport {
    conn: JsValue,
    sol: Object,
}

// impl Connection {
//     pub async fn get_account_data(&self, addr: &str) -> TransportResult<Vec<u8>> {
//         let sol = gloo::utils::window().get("solanaWeb3").unwrap();
//         let inner = &self.inner;
//         info!("Sol get");
//         let api = Reflect::get(&inner, &"getAccountInfo".into())
//             .unwrap() // unreachable
//             .dyn_into::<Function>()
//             .unwrap();
//         info!("api get");
//         let pubkey_type = Reflect::get(&sol, &"PublicKey".into())
//             .unwrap()
//             .dyn_into::<Function>()
//             .unwrap();
//         info!("pubkey get");
//         let pubkey_init_args = js_sys::Array::new();
//         info!("pubkey args");
//         pubkey_init_args.push(&addr.into());
//         let pubkey = Reflect::construct(&pubkey_type, &pubkey_init_args).unwrap();
//         info!("pubkey created");
//         api.bind(&self.inner);
//         let account_info = JsFuture::from(
//             api.call1(&JsValue::undefined(), &pubkey)
//                 .unwrap()
//                 .dyn_into::<Promise>()
//                 .unwrap(),
//         )
//         .await
//         .unwrap();
//         info!("account info get");
//         let data = Reflect::get(&account_info, &"data".into())
//             .unwrap()
//             .dyn_into::<Uint8Array>()
//             .unwrap();
//         info!("data get");
//         Ok(data.to_vec())
//     }
// }

#[async_trait(?Send)]
#[allow(unused)]
impl TransportLocalT for SolanaWasmTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        todo!()
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
        let pubkey = self.make_public_key(addr);
        debug!(format!("Get game account at {}", addr));
        let state: GameState = self.get_account_state(&pubkey).await?;
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
        let pubkey = self.make_public_key(addr);
        debug!(format!("Get player profile at {}", addr));
        let state: PlayerState = self.get_account_state(&pubkey).await?;
        Some(PlayerProfile {
            addr: addr.to_owned(),
            nick: state.nick,
            pfp: state.pfp.map(|p| p.to_string()),
        })
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        let pubkey = self.make_public_key(addr);
        debug!(format!("Get server profile at {}", addr));
        let state: ServerState = self.get_account_state(&pubkey).await?;
        Some(ServerAccount {
            addr: addr.to_owned(),
            owner_addr: state.owner.to_string(),
            endpoint: state.endpoint,
        })
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let pubkey = self.make_public_key(addr);
        debug!(format!("Get registration at {}", addr));
        let state: RegistryState = self.get_account_state(&pubkey).await?;
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
        let rpc = rpc.to_owned().into();
        let window = gloo::utils::window();
        let sol = window
            .get("solanaWeb3")
            .ok_or(TransportError::InitializationFailed(
                "solanaWeb3 not found".into(),
            ))?;
        let conn_ctor = Self::get_function(&sol, "Connection");
        let conn = Self::construct(&conn_ctor, &[&rpc]);
        info!("Solana Web3 Connection created:", &conn);
        Ok(Self { conn, sol })
    }

    fn get_function(obj: &JsValue, key: &str) -> Function {
        Reflect::get(obj, &key.into())
            .unwrap()
            .dyn_into::<Function>()
            .unwrap()
    }

    fn construct(ctor: &Function, args: &[&JsValue]) -> JsValue {
        let args = js_sys::Array::new();
        for arg in args.iter() {
            args.push(&arg);
        }
        Reflect::construct(ctor, &args).unwrap()
    }

    fn make_public_key(&self, addr: &str) -> JsValue {
        let pubkey_ctor = Self::get_function(&self.sol, "PublicKey");
        let new_pubkey_args = js_sys::Array::new();
        new_pubkey_args.push(&addr.clone().into());
        let pubkey = Reflect::construct(&pubkey_ctor, &new_pubkey_args).unwrap();
        pubkey
    }

    async fn resolve_promise(p: JsValue) -> Option<JsValue> {
        let p = match p.dyn_into::<Promise>() {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to resolve promise:", e);
                return None;
            }
        };
        match JsFuture::from(p).await {
            Ok(x) => Some(x),
            Err(e) => {
                warn!("Failed to resolve promise:", e);
                return None;
            }
        }
    }

    async fn get_account_state<T: BorshDeserialize>(&self, pubkey: &JsValue) -> Option<T> {
        let data = self.get_account_data(pubkey).await?;
        T::try_from_slice(&data).ok()
    }

    async fn get_account_data(&self, pubkey: &JsValue) -> Option<Vec<u8>> {
        let get_account_info = Self::get_function(&self.conn, "getAccountInfo");
        let p = match get_account_info.call1(&self.conn, pubkey) {
            Ok(p) => p,
            Err(e) => {
                warn!("Error when getting account data", e);
                return None;
            }
        };
        let account_info = Self::resolve_promise(p).await?;
        let data = match Reflect::get(&account_info, &"data".into()) {
            Ok(d) => d,
            Err(e) => {
                warn!("Error when getting account data, promise error", e);
                return None;
            }
        };

        let data = match data.dyn_into::<Uint8Array>() {
            Ok(d) => d,
            Err(e) => {
                warn!("Error when getting account data, promise error", e);
                return None;
            }
        };
        Some(data.to_vec())
    }
}

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
use gloo::console::{error, info};
use js_sys::{Function, Object, Promise, Reflect, Uint8Array};

use race_core::{
    error::Result,
    transport::TransportLocalT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, RegisterGameParams, RegistrationAccount, ServerAccount,
        UnregisterGameParams, VoteParams,
    },
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub struct SolanaWasmTransport {
    conn: Object,
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
        todo!()
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        todo!()
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        todo!()
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        None
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
        let conn_type = Reflect::get(&sol, &"Connection".into())
            .map_err(|e| {
                TransportError::InitializationFailed("Failed to get the ctor of Connection".into())
            })?
            .dyn_into::<Function>()
            .map_err(|e| {
                TransportError::InitializationFailed(
                    "Failed to cast the ctor of Connection to type Function".into(),
                )
            })?;
        let conn_new_args = js_sys::Array::new();
        conn_new_args.push(&rpc);
        let conn = Reflect::construct(&conn_type, &conn_new_args)
            .map_err(|_| {
                TransportError::InitializationFailed("Failed to initiate a Connection".into())
            })?
            .into();
        Ok(Self { conn, sol })
    }

    fn get_function(obj: &JsValue, key: &str) -> Function {
        Reflect::get(obj, &key.into()).unwrap().dyn_into::<Function>().unwrap()
    }

    fn make_public_key(&self, addr: String) -> JsValue {
        let pubkey_ctor = Self::get_function(&self.sol, "PublicKey");
        let new_pubkey_args = js_sys::Array::new();
        new_pubkey_args.push(&addr.clone().into());
        let pubkey = Reflect::construct(&pubkey_ctor, &new_pubkey_args).unwrap();
        pubkey
    }

    async fn get_account_data(&self, pubkey: &JsValue) -> Vec<u8> {
        let get_account_data = Reflect::get(&self.conn, &"getAccountData".into())
            .unwrap()
            .dyn_into::<Function>()
            .unwrap();
        vec![]
    }
}

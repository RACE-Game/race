use crate::error::Result;
use gloo::utils::format::JsValueSerdeExt;
use js_sys::{Object, Reflect, JSON::parse};
use race_core::{
    context::{GameContext, Player, Server},
    types::PlayerJoin,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Serialize)]
pub struct JsPlayer<'a> {
    pub addr: &'a str,
    pub position: usize,
    pub status: String,
    pub balance: u64,
}

impl<'a> From<&'a Player> for JsPlayer<'a> {
    fn from(value: &'a Player) -> Self {
        Self {
            addr: &value.addr,
            position: value.position,
            status: value.status.to_string(),
            balance: value.balance,
        }
    }
}

#[derive(Serialize)]
pub struct JsPlayerJoin<'a> {
    pub addr: &'a str,
    pub position: usize,
    pub balance: u64,
    pub access_version: u64,
}

impl<'a> From<&'a PlayerJoin> for JsPlayerJoin<'a> {
    fn from(value: &'a PlayerJoin) -> Self {
        Self {
            addr: &value.addr,
            position: value.position,
            balance: value.balance,
            access_version: value.access_version,
        }
    }
}

#[derive(Serialize)]
pub struct JsServer<'a> {
    pub addr: &'a str,
    pub status: String,
    pub endpoint: &'a str,
}

impl<'a> From<&'a Server> for JsServer<'a> {
    fn from(value: &'a Server) -> Self {
        Self {
            addr: &value.addr,
            status: value.status.to_string(),
            endpoint: &value.endpoint,
        }
    }
}

#[derive(Serialize)]
pub struct JsGameContext<'a> {
    pub game_addr: &'a str,
    pub access_version: u64,
    pub settle_version: u64,
    pub status: String,
    pub allow_exit: bool,
    pub players: Vec<JsPlayer<'a>>,
    pub pending_players: Vec<JsPlayerJoin<'a>>,
    pub servers: Vec<JsServer<'a>>,
}

impl<'a> JsGameContext<'a> {
    pub(crate) fn from_context(context: &'a GameContext) -> Self {
        Self {
            game_addr: context.get_game_addr(),
            access_version: context.get_access_version(),
            settle_version: context.get_settle_version(),
            status: context.get_status().to_string(),
            allow_exit: context.is_allow_exit(),
            servers: context.get_servers().iter().map(Into::into).collect(),
            players: context.get_players().iter().map(Into::into).collect(),
            pending_players: context
                .get_pending_players()
                .iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[wasm_bindgen]
pub struct Event {
    kind: String,
    sender: Option<String>,
    data: JsValue,
}

#[wasm_bindgen]
impl Event {
    #[wasm_bindgen]
    pub fn kind(&self) -> Result<String> {
        Ok(self.kind.clone())
    }

    #[wasm_bindgen]
    pub fn sender(&self) -> Option<String> {
        self.sender.clone()
    }

    #[wasm_bindgen]
    pub fn data(&self) -> JsValue {
        self.data.clone()
    }
}

impl From<race_core::event::Event> for Event {
    fn from(value: race_core::event::Event) -> Self {
        use race_core::event::Event::*;
        match value {
            Custom { sender, raw } => Self {
                kind: "custom".into(),
                sender: Some(sender),
                data: parse(&raw).unwrap(),
            },
            Ready { sender } => Self {
                kind: "ready".into(),
                sender: Some(sender),
                data: JsValue::null(),
            },
            ShareSecrets { sender, secrets } => {
                let data = Object::new();
                Reflect::set(
                    &data,
                    &"secrets".into(),
                    &JsValue::from_serde(&secrets).unwrap(),
                )
                .unwrap();
                Self {
                    kind: "share-secrets".into(),
                    sender: Some(sender),
                    data: JsValue::from(data),
                }
            }
            OperationTimeout { addr } => {
                let data = Object::new();
                Reflect::set(&data, &"addr".into(), &addr.into()).unwrap();
                Self {
                    kind: "operation-timeout".into(),
                    sender: None,
                    data: JsValue::from(data),
                }
            }
            Mask {
                sender,
                random_id,
                ciphertexts,
            } => {
                let data = Object::new();
                Reflect::set(&data, &"randomId".into(), &random_id.into()).unwrap();
                Reflect::set(
                    &data,
                    &"ciphertexts".into(),
                    &JsValue::from_serde(&ciphertexts).unwrap(),
                )
                .unwrap();
                Self {
                    kind: "mask".into(),
                    sender: Some(sender),
                    data: JsValue::from(data),
                }
            }
            Lock {
                sender,
                random_id,
                ciphertexts_and_digests,
            } => {
                let data = Object::new();
                Reflect::set(&data, &"randomId".into(), &random_id.into()).unwrap();
                Reflect::set(
                    &data,
                    &"ciphertextsAndDigests".into(),
                    &JsValue::from_serde(&ciphertexts_and_digests).unwrap(),
                )
                .unwrap();
                Self {
                    kind: "lock".into(),
                    sender: Some(sender),
                    data: JsValue::from(data),
                }
            }
            RandomnessReady { random_id } => {
                let data = Object::new();
                Reflect::set(&data, &"randomId".into(), &random_id.into()).unwrap();
                Self {
                    kind: "randomness-ready".into(),
                    sender: None,
                    data: JsValue::from(data),
                }
            }
            Sync {
                new_players,
                new_servers,
                transactor_addr,
                access_version,
            } => {
                let data = Object::new();
                Reflect::set(&data, &"transactorAddr".into(), &transactor_addr.into()).unwrap();
                Reflect::set(
                    &data,
                    &"newPlayers".into(),
                    &JsValue::from_serde(&new_players).unwrap(),
                )
                .unwrap();
                Reflect::set(
                    &data,
                    &"newServers".into(),
                    &JsValue::from_serde(&new_servers).unwrap(),
                )
                .unwrap();
                Reflect::set(&data, &"accessVersion".into(), &access_version.into()).unwrap();
                Self {
                    kind: "sync".into(),
                    sender: None,
                    data: JsValue::from(data),
                }
            }
            ServerLeave {
                server_addr,
                transactor_addr,
            } => {
                let data = Object::new();
                Reflect::set(&data, &"transactorAddr".into(), &transactor_addr.into()).unwrap();
                Reflect::set(&data, &"serverAddr".into(), &server_addr.into()).unwrap();
                Self {
                    kind: "server-leave".into(),
                    sender: None,
                    data: JsValue::from(data),
                }
            }
            Leave { player_addr } => {
                let data = Object::new();
                Reflect::set(&data, &"playerAddr".into(), &player_addr.into()).unwrap();
                Self {
                    kind: "leave".into(),
                    sender: None,
                    data: JsValue::from(data),
                }
            }
            GameStart { access_version } => {
                let data = Object::new();
                Reflect::set(&data, &"accessVersion".into(), &access_version.into()).unwrap();
                Self {
                    kind: "game-start".into(),
                    sender: None,
                    data: JsValue::from(data),
                }
            }
            WaitingTimeout => Self {
                kind: "waiting-timeout".into(),
                sender: None,
                data: JsValue::null(),
            },
            ActionTimeout { player_addr } => {
                let data = Object::new();
                Reflect::set(&data, &"playerAddr".into(), &player_addr.into()).unwrap();
                Self {
                    kind: "action-timeout".into(),
                    sender: None,
                    data: JsValue::from(data),
                }
            }
            SecretsReady => Self {
                kind: "secrets-ready".into(),
                sender: None,
                data: JsValue::null(),
            },
            Shutdown => Self {
                kind: "shutdown".into(),
                sender: None,
                data: JsValue::null(),
            },
            _ => Self {
                kind: "unknown".into(),
                sender: None,
                data: JsValue::null(),
            },
        }
    }
}

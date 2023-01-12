//! Parameters for interacting with transactor

use serde::{Deserialize, Serialize};
use crate::event::Event;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachGameParams {
    pub addr: String,
    pub chain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStateParams {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetContextParams {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeEventParams {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitEventParams {
    pub addr: String,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastFrame {
    pub game_addr: String,
    pub state_json: String,
    pub event: Event,
}

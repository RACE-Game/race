//! Parameters for interacting with transactor

use std::fmt::Display;

use crate::event::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachGameParams {
    pub key: String,
}

impl Display for AttachGameParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AttachGameParams")
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitEventParams {
    pub event: Event,
}

impl Display for SubmitEventParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubmitEventParams")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitGameParams {}

impl Display for ExitGameParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExitGameParams")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStateParams {}

impl Display for GetStateParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetStateParams")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrieveEventsParams {
    pub settle_version: u64,
}

impl Display for RetrieveEventsParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetStateParams")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeEventParams {
    pub settle_version: u64,
}

impl Display for SubscribeEventParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubscribeEventParams")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BroadcastFrame {
    pub game_addr: String,
    pub event: Event,
    pub timestamp: u64,
}

impl Display for BroadcastFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BroadcastFrame: {}", self.event)
    }
}

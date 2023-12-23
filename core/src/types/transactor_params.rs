//! Parameters for interacting with transactor

use crate::encryptor::NodePublicKeyRaw;
use crate::types::PlayerJoin;
use borsh::{BorshDeserialize, BorshSerialize};
use race_api::{
    event::{Event, Message},
    types::ServerJoin,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum TxState {
    PlayerConfirming {
        confirm_players: Vec<PlayerJoin>,
        access_version: u64,
    },

    PlayerConfirmingFailed(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AttachGameParams {
    pub signer: String,
    pub key: NodePublicKeyRaw,
}

impl Display for AttachGameParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AttachGameParams")
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SubmitEventParams {
    pub event: Event,
}

impl Display for SubmitEventParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubmitEventParams")
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SubmitMessageParams {
    pub content: String,
}

impl Display for SubmitMessageParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubmitMessageParams")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ExitGameParams {}

impl Display for ExitGameParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExitGameParams")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SubscribeEventParams {
    pub settle_version: u64,
}

impl Display for SubscribeEventParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubscribeEventParams")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NodeJoin {
    pub addr: String,
    pub access_version: u64,
}

impl From<ServerJoin> for NodeJoin {
    fn from(value: ServerJoin) -> Self {
        Self {
            addr: value.addr,
            access_version: value.access_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum BroadcastFrame {
    // Game event
    Event {
        game_addr: String,
        event: Event,
        timestamp: u64,
    },
    // Arbitrary message
    Message {
        game_addr: String,
        message: Message,
    },
    // Transaction state updates
    TxState {
        tx_state: TxState,
    },
    // Node state updates
    UpdateNodes {
        nodes: Vec<NodeJoin>,
        transactor_addr: Option<String>,
    },
}

impl Display for BroadcastFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BroadcastFrame::Event { event, .. } => {
                write!(f, "BroadcastFrame::Event: {}", event)
            }
            BroadcastFrame::Message { message, .. } => {
                write!(f, "BroadcastFrame::Message: {}", message.sender)
            }
            BroadcastFrame::TxState { tx_state } => {
                write!(f, "BroadcastFrame::TxState: {:?}", tx_state)
            }
            BroadcastFrame::UpdateNodes {
                nodes,
                transactor_addr,
            } => {
                write!(
                    f,
                    "BroadcastFrame::UpdateNodes: {:?}, current transactor: {:?}",
                    nodes, transactor_addr
                )
            }
        }
    }
}

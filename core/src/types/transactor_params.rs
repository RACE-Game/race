//! Parameters for interacting with transactor

use std::fmt::Display;
use race_api::event::{Event, Message};
use crate::{
    encryptor::NodePublicKeyRaw,
};
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::types::PlayerJoin;

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
pub enum BroadcastFrame {
    Event {
        game_addr: String,
        event: Event,
        timestamp: u64,
    },
    Init {
        game_addr: String,
        access_version: u64,
        settle_version: u64,
        checkpoint_state: Vec<u8>,
    },
    Message {
        game_addr: String,
        message: Message,
    },
    TxState {
        tx_state: TxState,
    }
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
            BroadcastFrame::Init {
                access_version,
                settle_version,
                ..
            } => {
                write!(
                    f,
                    "BroadcastFrame::Init, access: {}, settle: {}",
                    access_version, settle_version
                )
            }
            BroadcastFrame::TxState {
                tx_state,
            } => {

                write!(
                    f,
                    "BroadcastFrame::TxState: {:?}",
                    tx_state
                )
            }
         }
    }
}

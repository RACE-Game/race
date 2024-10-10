use crate::checkpoint::CheckpointWithProof;
use crate::types::PlayerJoin;
use borsh::{BorshDeserialize, BorshSerialize};
use race_api::event::{Event, Message};
use race_api::types::ServerJoin;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use super::TxState;

#[derive(Default, Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BroadcastSync {
    pub new_players: Vec<PlayerJoin>,
    pub new_servers: Vec<ServerJoin>,
    pub transactor_addr: String,
    pub access_version: u64,
}

impl BroadcastSync {
    pub fn new(access_version: u64) -> Self {
        Self {
            access_version,
            ..Default::default()
        }
    }

    pub fn merge(&mut self, other: &Self) {
        self.new_players.append(&mut other.new_players.clone());
        self.new_servers.append(&mut other.new_servers.clone());
        self.access_version = u64::max(self.access_version, other.access_version);
        self.transactor_addr = other.transactor_addr.clone();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EventHistory {
    pub event: Event,
    pub timestamp: u64,
    pub state_sha: String,
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
        state_sha: String,
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
    Sync {
        sync: BroadcastSync,
    },
    // This frame is the first frame in broadcast stream.
    EventHistories {
        game_addr: String,
        checkpoint_with_proof: Option<CheckpointWithProof>,
        histories: Vec<EventHistory>,
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
            BroadcastFrame::Sync { sync } => {
                write!(
                    f,
                    "BroadcastFrame::Sync: access_version {}",
                    sync.access_version
                )
            }
            BroadcastFrame::EventHistories { histories, .. } => {
                write!(f, "BroadcastFrame::EventHistories, len: {}", histories.len())
            }
        }
    }
}

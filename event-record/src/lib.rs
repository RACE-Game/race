//! The records file is saved with following layout.
//!
//! [RecordsHeader] - the game address and bundle address
//! [Record]*       - One record on each line
//!
//! All content are serialized with borsh & base64.

use borsh::{BorshSerialize, BorshDeserialize};
use race_core::context::Node;
use race_api::event::Event;

#[derive(Default, Debug, BorshSerialize, BorshDeserialize)]
pub struct RecordsHeader {
    pub game_addr: String,
    pub game_id: usize,
    pub bundle_addr: String,
    pub chain: String,        // solana, sui, facade
}

impl RecordsHeader {
    pub fn new(game_addr: String, game_id: usize, bundle_addr: String, chain: String) -> Self {
        Self { game_addr, game_id, bundle_addr, chain }
    }
}

impl std::fmt::Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Record::Checkpoint { .. } => {
                write!(f, "<Checkpoint>")
            }
            Record::Event { event, .. } => {
                write!(f, "{}", event)
            }
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum Record {
    Checkpoint {
        state: Vec<u8>,
        nodes: Vec<Node>,
    },
    Event {
        event: Event,
        timestamp: u64,
    }
}

impl Record {
    pub fn checkpoint(state: Vec<u8>, nodes: Vec<Node>) -> Self {
        Self::Checkpoint {
            state, nodes,
        }
    }

    pub fn event(event: Event, timestamp: u64) -> Self {
        Self::Event {
            event, timestamp,
        }
    }
}

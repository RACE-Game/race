//! The records file is saved with following layout.
//!
//! [RecordsHeader] - the game address and bundle address
//! [Record]*       - One record on each line
//!
//! All content are serialized with borsh & base64.

use borsh::{BorshSerialize, BorshDeserialize};
use race_core::{context::Node, types::{PlayerBalance, GameSpec, EntryType}};
use race_api::event::Event;

#[derive(Default, Debug, BorshSerialize, BorshDeserialize)]
pub struct RecordsHeader {
    pub spec: GameSpec,
    pub chain: String,        // solana, sui, facade
    pub init_data: Vec<u8>,
    pub entry_type: EntryType,
}

impl RecordsHeader {
    pub fn new(spec: GameSpec, init_data: Vec<u8>, entry_type: EntryType, chain: String) -> Self {
        Self { spec, init_data, entry_type, chain }
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
        balances: Vec<PlayerBalance>,
        access_version: u64,
        settle_version: u64,
    },
    Event {
        event: Event,
        timestamp: u64,
    }
}

impl Record {
    pub fn checkpoint(
        state: Vec<u8>,
        nodes: Vec<Node>,
        balances: Vec<PlayerBalance>,
        access_version: u64,
        settle_version: u64
    ) -> Self {
        Self::Checkpoint {
            state, nodes, balances, access_version, settle_version
        }
    }

    pub fn event(event: Event, timestamp: u64) -> Self {
        Self::Event {
            event, timestamp,
        }
    }
}

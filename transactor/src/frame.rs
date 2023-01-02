use race_core::{types::{Player, SettleParams}, event::Event, context::GameContext};
use borsh::{BorshSerialize, BorshDeserialize};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum EventFrame {
    Empty,
    PlayerJoined {
        addr: String,
        players: Vec<Player>,
    },
    SendEvent {
        addr: String,
        event: Event,
    },
    Broadcast {
        addr: String,
        state_json: String,
        event: Event,
    },
    Settle {
        addr: String,
        params: SettleParams,
    },
    ContextUpdated {
        context: GameContext,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastFrame {
    pub game_addr: String,
    pub state_json: String,
    pub event: Event,
}

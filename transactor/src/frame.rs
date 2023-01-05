use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    context::GameContext,
    event::Event,
    types::{Player, SettleParams},
};
use serde::{Deserialize, Serialize};

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
    SendServerEvent {
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

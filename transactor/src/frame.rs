use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{context::GameContext, event::Event, types::SettleParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct NewPlayer {
    pub addr: String,
    pub position: usize,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum EventFrame {
    Empty,
    PlayerJoined { new_players: Vec<NewPlayer> },
    PlayerDeposited { player_addr: String, amount: u64 },
    SendEvent { event: Event },
    SendServerEvent { event: Event },
    Broadcast { state_json: String, event: Event },
    Settle { params: SettleParams },
    ContextUpdated { context: GameContext },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastFrame {
    pub game_addr: String,
    pub state_json: String,
    pub event: Event,
}

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{context::GameContext, event::Event, types::Settle};

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct NewPlayer {
    pub addr: String,
    pub position: usize,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct NewServer {
    pub addr: String,
    pub endpoint: String,
}

#[derive(Debug, Clone)]
pub enum SignalFrame {
    StartGame { game_addr: String },
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum EventFrame {
    Empty,
    PlayerJoined {
        new_players: Vec<NewPlayer>,
    },
    ServerJoined {
        new_servers: Vec<NewServer>,
        transactor_addr: String,
    },
    PlayerDeposited {
        player_addr: String,
        amount: u64,
    },
    PlayerLeaving {
        player_addr: String,
    },
    SendEvent {
        event: Event,
    },
    SendServerEvent {
        event: Event,
    },
    Broadcast {
        state_json: String,
        event: Event,
        access_version: u64,
        settle_version: u64,
    },
    Settle {
        settles: Vec<Settle>,
    },
    SettleFinalized {
        settle_version: u64,
    },
    ContextUpdated {
        context: GameContext,
    },
    Shutdown,
}

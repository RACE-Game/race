use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{context::GameContext, event::Event, types::{Settle, NewPlayer, NewServer}};

#[derive(Debug, Clone)]
pub enum SignalFrame {
    StartGame { game_addr: String },
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum EventFrame {
    Empty,
    Sync {
        new_players: Vec<NewPlayer>,
        new_servers: Vec<NewServer>,
        transactor_addr: String,
        access_version: u64,
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

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    context::GameContext,
    event::Event,
    types::{PlayerJoin, ServerJoin, Settle},
};

#[derive(Debug, Clone)]
pub enum SignalFrame {
    StartGame { game_addr: String },
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum EventFrame {
    Empty,
    Sync {
        new_players: Vec<PlayerJoin>,
        new_servers: Vec<ServerJoin>,
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

impl std::fmt::Display for EventFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventFrame::Empty => write!(f, "Empty"),
            EventFrame::Sync {
                new_players,
                new_servers,
                transactor_addr,
                access_version,
            } => write!(
                f,
                "Sync, new players: {}, new servers: {}, access version = {}",
                new_players.len(),
                new_servers.len(),
                access_version
            ),
            EventFrame::PlayerDeposited {
                player_addr,
                amount,
            } => write!(f, "PlayerDeposited"),
            EventFrame::PlayerLeaving { player_addr } => write!(f, "PlayerLeaving"),
            EventFrame::SendEvent { event } => write!(f, "SendEvent: {}", event),
            EventFrame::SendServerEvent { event } => write!(f, "SendServerEvent: {}", event),
            EventFrame::Broadcast {
                state_json,
                event,
                access_version,
                settle_version,
            } => write!(f, "Broadcast: {}", event),
            EventFrame::Settle { settles } => write!(f, "Settle"),
            EventFrame::SettleFinalized { settle_version } => write!(f, "SettleFinalized"),
            EventFrame::ContextUpdated { context: _ } => write!(f, "ContextUpdated"),
            EventFrame::Shutdown => write!(f, "Shutdown"),
        }
    }
}

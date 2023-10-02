use borsh::{BorshDeserialize, BorshSerialize};
use race_api::{
    engine::InitAccount,
    event::{Event, Message},
};
use race_core::{
    context::GameContext,
    types::{PlayerJoin, ServerJoin, Settle, Transfer, TxState, VoteType},
};

#[derive(Debug, Clone)]
pub enum SignalFrame {
    StartGame { game_addr: String },
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum EventFrame {
    Empty,
    Sync {
        new_players: Vec<PlayerJoin>,
        new_servers: Vec<ServerJoin>,
        transactor_addr: String,
        access_version: u64,
    },
    TxState {
        tx_state: TxState,
    },
    PlayerDeposited {
        player_addr: String,
        amount: u64,
    },
    PlayerLeaving {
        player_addr: String,
    },
    InitState {
        init_account: InitAccount,
    },
    SendEvent {
        event: Event,
    },
    SendMessage {
        message: Message,
    },
    SendServerEvent {
        event: Event,
    },
    Checkpoint {
        access_version: u64,
        settle_version: u64,
    },
    Broadcast {
        event: Event,
        access_version: u64,
        settle_version: u64,
        timestamp: u64,
    },
    Settle {
        settles: Vec<Settle>,
        transfers: Vec<Transfer>,
        checkpoint: Vec<u8>,
    },
    SettleFinalized {
        settle_version: u64,
    },
    ContextUpdated {
        context: GameContext,
    },
    Vote {
        votee: String,
        vote_type: VoteType,
    },
    Shutdown,
}

impl std::fmt::Display for EventFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventFrame::Empty => write!(f, "Empty"),
            EventFrame::InitState { init_account, .. } => write!(
                f,
                "InitState, access_version = {}, settle_version = {}",
                init_account.access_version, init_account.settle_version
            ),
            EventFrame::Sync {
                new_players,
                new_servers,
                access_version,
                ..
            } => write!(
                f,
                "Sync, new players: {}, new servers: {}, access version = {}",
                new_players.len(),
                new_servers.len(),
                access_version
            ),
            EventFrame::TxState { tx_state: _ } => write!(
                f,
                "TxState",
                // confirm_players.len(),
                // access_version,
                // confirm_success,
            ),
            EventFrame::PlayerDeposited { .. } => write!(f, "PlayerDeposited"),
            EventFrame::PlayerLeaving { .. } => write!(f, "PlayerLeaving"),
            EventFrame::SendEvent { event } => write!(f, "SendEvent: {}", event),
            EventFrame::SendServerEvent { event } => write!(f, "SendServerEvent: {}", event),
            EventFrame::Checkpoint { .. } => write!(f, "Checkpoint"),
            EventFrame::Broadcast { event, .. } => write!(f, "Broadcast: {}", event),
            EventFrame::Settle { .. } => write!(f, "Settle"),
            EventFrame::SettleFinalized { .. } => write!(f, "SettleFinalized"),
            EventFrame::SendMessage { message } => write!(f, "SendMessage: {}", message.sender),
            EventFrame::ContextUpdated { context: _ } => write!(f, "ContextUpdated"),
            EventFrame::Shutdown => write!(f, "Shutdown"),
            EventFrame::Vote { votee, vote_type } => {
                write!(f, "Vote: to {} for {:?}", votee, vote_type)
            }
        }
    }
}

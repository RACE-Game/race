use borsh::{BorshDeserialize, BorshSerialize};
use race_api::{
    init_account::InitAccount,
    event::{Event, Message}
};
use race_core::{
    context::GameContext,
    types::{PlayerJoin, ServerJoin, SettleWithAddr, SubGameSpec, Transfer, TxState, VoteType}, checkpoint::Checkpoint,
};

#[derive(Debug, Clone)]
pub enum SignalFrame {
    StartGame { game_addr: String },
    LaunchSubGame { spec: SubGameSpec, checkpoint: Checkpoint },
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum EventFrame {
    Empty,
    GameStart {
        access_version: u64,
    },
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
        access_version: u64,
        settle_version: u64,
        init_account: InitAccount,
        checkpoint: Checkpoint,
    },
    SendEvent {
        event: Event,
        timestamp: u64,
    },
    SendMessage {
        message: Message,
    },
    SendServerEvent {
        event: Event,
        timestamp: u64,
    },
    Checkpoint {
        settles: Vec<SettleWithAddr>,
        transfers: Vec<Transfer>,
        checkpoint: Checkpoint,
        access_version: u64,
        settle_version: u64,
        previous_settle_version: u64,
        state_sha: String,
    },
    Broadcast {
        event: Event,
        access_version: u64,
        settle_version: u64,
        timestamp: u64,
        state_sha: String,
    },
    ContextUpdated {
        context: Box<GameContext>,
    },
    Vote {
        votee: String,
        vote_type: VoteType,
    },
    Shutdown,
    /// Represent a event send in current event bus.  `from` is the
    /// source of event, `dest` is the target of the event.  value 0
    /// represent the master game.  When there's an available
    /// checkpoint in the context, it will be sent along with
    /// `checkpoint`.
    SendBridgeEvent {
        from: usize,
        dest: usize,
        event: Event,
        access_version: u64,
        settle_version: u64,
        checkpoint_state: Vec<u8>,
    },
    /// Similar to `SendBridgeEvent`, but for receiver's event bus.
    RecvBridgeEvent {
        from: usize,
        dest: usize,
        event: Event,
        access_version: u64,
        settle_version: u64,
        checkpoint_state: Vec<u8>,
    },
    LaunchSubGame {
        spec: Box<SubGameSpec>,
        checkpoint: Checkpoint,
    },
}

impl std::fmt::Display for EventFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventFrame::Empty => write!(f, "Empty"),
            EventFrame::GameStart { access_version } => {
                write!(f, "GameStart, access_version = {}", access_version)
            }
            EventFrame::InitState { access_version, settle_version, .. } => write!(
                f,
                "InitState, access_version = {}, settle_version = {}",
                access_version, settle_version
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
            EventFrame::SendEvent { event, .. } => write!(f, "SendEvent: {}", event),
            EventFrame::SendServerEvent { event, .. } => write!(f, "SendServerEvent: {}", event),
            EventFrame::Checkpoint { .. } => write!(f, "Checkpoint"),
            EventFrame::Broadcast { event, .. } => write!(f, "Broadcast: {}", event),
            EventFrame::SendMessage { message } => write!(f, "SendMessage: {}", message.sender),
            EventFrame::ContextUpdated { context: _ } => write!(f, "ContextUpdated"),
            EventFrame::Shutdown => write!(f, "Shutdown"),
            EventFrame::Vote { votee, vote_type } => {
                write!(f, "Vote: to {} for {:?}", votee, vote_type)
            }
            EventFrame::SendBridgeEvent { dest, event, settle_version, .. } => {
                write!(f, "SendBridgeEvent: dest {}, settle_version: {}, event: {}", dest, settle_version, event)
            }
            EventFrame::RecvBridgeEvent { dest, event, settle_version, .. } => {
                write!(f, "RecvBridgeEvent: dest {}, settle_version: {}, event: {}", dest, settle_version, event)
            }
            EventFrame::LaunchSubGame { spec, .. } => {
                write!(f, "LaunchSubGame: {:?}", spec)
            }
        }
    }
}

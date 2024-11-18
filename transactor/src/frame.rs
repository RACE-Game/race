use race_api::{
    event::{Event, Message}, types::{EntryLock, Settle}
};
use race_core::{
    checkpoint::Checkpoint, context::{GameContext, SubGameInit}, types::{PlayerDeposit, PlayerJoin, ServerJoin, Transfer, TxState, VoteType}
};

#[derive(Debug, Clone)]
pub enum SignalFrame {
    StartGame { game_addr: String },
    LaunchSubGame { sub_game_init: SubGameInit },
}

#[derive(Debug, Clone)]
pub enum EventFrame {
    #[allow(unused)]
    Empty,
    GameStart {
        access_version: u64,
    },
    Sync {
        new_players: Vec<PlayerJoin>,
        new_servers: Vec<ServerJoin>,
        new_deposits: Vec<PlayerDeposit>,
        transactor_addr: String,
        access_version: u64,
    },
    TxState {
        tx_state: TxState,
    },
    PlayerLeaving {
        player_addr: String,
    },
    InitState {
        access_version: u64,
        settle_version: u64,
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
        settles: Vec<Settle>,
        transfers: Vec<Transfer>,
        checkpoint: Checkpoint,
        access_version: u64,
        settle_version: u64,
        previous_settle_version: u64,
        state_sha: String,
        entry_lock: Option<EntryLock>,
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

    /// Launch a subgame.
    LaunchSubGame {
        sub_game_init: Box<SubGameInit>,
    },

    /// Resume a subgame from its checkpoint.
    #[allow(unused)]
    ResumeSubGame {
        checkpoint: Checkpoint,
    },

    /// Sync frame for subgames broadcasted from master game.
    SubSync {
        new_players: Vec<PlayerJoin>,
        new_servers: Vec<ServerJoin>,
        transactor_addr: String,
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
            EventFrame::LaunchSubGame { sub_game_init } => {
                write!(f, "LaunchSubGame: {}#{}", sub_game_init.spec.game_addr, sub_game_init.spec.game_id)
            }
            EventFrame::SubSync { new_players, new_servers, .. } => {
                write!(f, "SyncNodes: new_players: {}, new_servers: {}", new_players.len(), new_servers.len())
            },
            EventFrame::ResumeSubGame { .. } => {
                write!(f, "ResumeSubGame")
            }
        }
    }
}

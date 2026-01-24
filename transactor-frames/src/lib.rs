use tokio::sync::{mpsc, broadcast};

use race_api::event::{Event, Message};
use race_api::init_account::InitAccount;
use race_core::node::Node;
use race_core::context::{GameContext, SettleDetails};
use race_core::checkpoint::{ContextCheckpoint, VersionedData};
use race_core::types::{ClientMode, PlayerDeposit, PlayerJoin, ServerJoin, TxState, VoteType};

#[derive(Debug)]
pub struct BridgeToParent {
    pub tx_to_parent: mpsc::Sender<EventFrame>,
    pub rx_from_parent: broadcast::Receiver<EventFrame>,
}

#[derive(Debug)]
pub enum SignalFrame {
    StartGame {
        game_addr: String,
        mode: ClientMode,
    },
    LaunchSubGame {
        checkpoint: ContextCheckpoint,
        bridge_to_parent: BridgeToParent,
    },
    #[allow(unused)]
    Shutdown,
    RemoveGame {
        game_addr: String,
    },
}

#[derive(Debug, Clone)]
pub enum EventFrame {
    #[allow(unused)]
    Empty,
    Sync {
        new_players: Vec<PlayerJoin>,
        new_servers: Vec<ServerJoin>,
        new_deposits: Vec<PlayerDeposit>,
        transactor_addr: String,
        access_version: u64,
    },
    // Send when credentials of players & servers are loaded
    SyncWithCredentials {
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
    RecoverCheckpoint {
        checkpoint: ContextCheckpoint,
    },
    RecoverCheckpointWithCredentials {
        checkpoint: ContextCheckpoint,
    },
    InitState {
        access_version: u64,
        settle_version: u64,
        init_account: InitAccount,
        nodes: Vec<Node>,
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
        checkpoint: ContextCheckpoint,
    },
    Settle {
        settle_details: Box<SettleDetails>,
    },
    Broadcast {
        event: Event,
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
        versioned_data: VersionedData,
    },
    /// Similar to `SendBridgeEvent`, but for receiver's event bus.
    RecvBridgeEvent {
        from: usize,
        dest: usize,
        event: Event,
        versioned_data: VersionedData,
    },

    /// Launch a subgame.
    LaunchSubGame {
        checkpoint: Box<ContextCheckpoint>,
    },

    /// Sync frame for subgames broadcasted from master game.
    SubSync {
        access_version: u64,
        new_players: Vec<PlayerJoin>,
        new_servers: Vec<ServerJoin>,
        transactor_addr: String,
    },

    /// Subgames send this frame after they made their first checkpoint.
    SubGameReady {
        game_id: usize,
        versioned_data: VersionedData,
        max_players: u16,
        init_data: Vec<u8>,
    },

    /// Notify that the subgame is launched successfully
    SubGameLaunched {
        game_id: usize,
    },

    /// Subgame send this frame when it will be shutdown.
    SubGameShutdown {
        game_id: usize,
        versioned_data: VersionedData,
    },

    /// Reject a deposit
    RejectDeposits {
        reject_deposits: Vec<u64>,
    },
}

impl std::fmt::Display for EventFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventFrame::Empty => write!(f, "Empty"),
            EventFrame::InitState {
                access_version,
                settle_version,
                ..
            } => write!(
                f,
                "InitState, access_version = {}, settle_version = {}",
                access_version, settle_version
            ),
            EventFrame::RecoverCheckpoint {
                ..
            } => write!(
                f,
                "RecoverCheckpoint",
            ),
            EventFrame::RecoverCheckpointWithCredentials {
                ..
            } => write!(
                f,
                "RecoverCheckpointWithCredentials",
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
            EventFrame::SyncWithCredentials {
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
            EventFrame::Settle { .. } => write!(f, "Settle"),
            EventFrame::Checkpoint { .. } => write!(f, "Checkpoint"),
            EventFrame::Broadcast { event, .. } => write!(f, "Broadcast: {}", event),
            EventFrame::SendMessage { message } => write!(f, "SendMessage: {}", message.sender),
            EventFrame::ContextUpdated { context: _ } => write!(f, "ContextUpdated"),
            EventFrame::Shutdown => write!(f, "Shutdown"),
            EventFrame::Vote { votee, vote_type } => {
                write!(f, "Vote: to {} for {:?}", votee, vote_type)
            }
            EventFrame::SendBridgeEvent { dest, event, .. } => {
                write!(f, "SendBridgeEvent: dest {}, event: {}", dest, event)
            }
            EventFrame::RecvBridgeEvent { dest, event, .. } => {
                write!(f, "RecvBridgeEvent: dest {}, event: {}", dest, event)
            }
            EventFrame::LaunchSubGame { checkpoint } => {
                write!(
                    f,
                    "LaunchSubGame: {}#{}",
                    checkpoint.root_data.game_spec.game_addr,
                    checkpoint.root_data.game_spec.game_id,
                )
            }
            EventFrame::SubSync {
                new_players,
                new_servers,
                ..
            } => {
                write!(
                    f,
                    "SyncNodes: new_players: {}, new_servers: {}",
                    new_players.len(),
                    new_servers.len()
                )
            }
            EventFrame::SubGameReady { game_id, .. } => {
                write!(f, "SubGameReady, game_id: {}", game_id)
            }
            EventFrame::SubGameLaunched { game_id } => {
                write!(f, "SubGameLaunched, game_id: {}", game_id)
            }
            EventFrame::SubGameShutdown { game_id, .. } => {
                write!(f, "SubGameShutdown, game_id: {}", game_id)
            }
            EventFrame::RejectDeposits { reject_deposits } => {
                write!(f, "Reject deposits, {:?}", reject_deposits)
            }
        }
    }
}

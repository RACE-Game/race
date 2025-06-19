use race_api::{
    event::{Event, Message},
    types::GameId,
};
use race_core::{
    checkpoint::{Checkpoint, VersionedData},
    context::{GameContext, SettleDetails, SubGameInit},
    types::{ClientMode, PlayerDeposit, PlayerJoin, ServerJoin, TxState, VoteType},
};

use crate::component::BridgeToParent;

#[derive(Debug)]
pub enum SignalFrame {
    StartGame {
        game_addr: String,
        mode: ClientMode,
    },
    LaunchSubGame {
        sub_game_init: SubGameInit,
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
        checkpoint: Checkpoint,
        access_version: u64,
        settle_version: u64,
        state_sha: String,
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
        from: GameId,
        dest: GameId,
        event: Event,
        // access_version: u64,
        // settle_version: u64,
        versioned_data: VersionedData,
    },
    /// Similar to `SendBridgeEvent`, but for receiver's event bus.
    RecvBridgeEvent {
        from: GameId,
        dest: GameId,
        event: Event,
        // #[allow(unused)]
        // access_version: u64,
        // settle_version: u64,
        versioned_data: VersionedData,
    },

    /// Launch a subgame.
    LaunchSubGame {
        sub_game_init: Box<SubGameInit>,
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
        game_id: GameId,
        versioned_data: VersionedData,
        max_players: u16,
        init_data: Vec<u8>,
    },

    /// Subgames send this frame when start via recovering from checkpoint.
    SubGameRecovered {
        game_id: GameId,
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
            EventFrame::GameStart { access_version } => {
                write!(f, "GameStart, access_version = {}", access_version)
            }
            EventFrame::InitState {
                access_version,
                settle_version,
                ..
            } => write!(
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
            EventFrame::LaunchSubGame { sub_game_init } => {
                write!(
                    f,
                    "LaunchSubGame: {}#{}",
                    sub_game_init.spec.game_addr, sub_game_init.spec.game_id
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
            EventFrame::SubGameRecovered { game_id } => {
                write!(f, "SubGameRecovered, game_id: {}", game_id)
            }
            EventFrame::RejectDeposits { reject_deposits } => {
                write!(f, "Reject deposits, {:?}", reject_deposits)
            }
        }
    }
}

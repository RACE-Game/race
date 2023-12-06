use crate::{
    error::HandleError,
    types::{Ciphertext, DecisionId, PlayerJoin, RandomId, SecretDigest, SecretShare, ServerJoin},
};
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A message sent by player
/// Used to express unimportant game events that
/// can be sent at any time without the server checking
/// their content.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Message {
    pub sender: String,
    pub content: String,
}

/// Game event structure
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Event {
    /// Sent by player clients.  Represent game specific events, the `raw`
    /// parts is the serialized data from a custom game event which
    /// satisfies [`CustomEvent`].
    Custom {
        sender: String,
        raw: Vec<u8>,
    },

    /// A event sent by system, the first event sent by transactor
    /// when game is loaded.
    Ready,

    /// Transactor shares its secert to specific player.
    /// The `secret_data` is encrypted with the receiver's public key.
    ShareSecrets {
        sender: String,
        shares: Vec<SecretShare>,
    },

    OperationTimeout {
        addrs: Vec<String>,
    },

    /// Randomize items.
    /// This event is sent by transactors.
    Mask {
        sender: String,
        random_id: RandomId,
        ciphertexts: Vec<Ciphertext>,
    },

    /// Lock items.
    /// This event is sent by transactors.
    Lock {
        sender: String,
        random_id: RandomId,
        ciphertexts_and_digests: Vec<(Ciphertext, SecretDigest)>,
    },

    /// All randomness is prepared.
    /// This event is sent by transactor.
    RandomnessReady {
        random_id: RandomId,
    },

    /// Sync with on-chain account.  New players/servers will be added frist to
    /// game context and then to game handler (WASM).
    /// This event is sent by transactor based on the diff of the account states.
    Sync {
        new_players: Vec<PlayerJoin>,
        new_servers: Vec<ServerJoin>,
        transactor_addr: String,
        access_version: u64,
    },

    /// A server left the game.
    /// `transactor_addr` is the new current transactor address.
    ///
    /// NOTE: This event must be handled idempotently.
    ServerLeave {
        server_addr: String,
        transactor_addr: String,
    },

    /// Client left game
    /// This event is sent by transactor based on client's connection status.
    Leave {
        player_addr: String,
    },

    /// Transactor uses this event as the start for each game.
    /// The `access_version` can be used to filter out which players are included.
    GameStart {
        access_version: u64,
    },

    /// Timeout when waiting for start
    WaitingTimeout,

    /// Random drawer takes random items by indexes.
    DrawRandomItems {
        sender: String,
        random_id: usize,
        indexes: Vec<usize>,
    },

    /// Timeout for drawing random items
    DrawTimeout,

    /// Timeout when waiting for player's action
    /// Sent by transactor.
    ActionTimeout {
        player_addr: String,
    },

    /// Answer the decision question with encrypted ciphertext
    AnswerDecision {
        sender: String,
        decision_id: DecisionId,
        ciphertext: Ciphertext,
        digest: SecretDigest,
    },

    /// All required secrets are shared
    SecretsReady {
        random_ids: Vec<usize>,
    },

    /// Shutdown
    Shutdown,
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Custom { sender, raw } => write!(f, "Custom from {}, inner: {:?}", sender, raw),
            Event::Ready => write!(f, "Ready"),
            Event::ShareSecrets { sender, shares } => {
                let repr = shares
                    .iter()
                    .map(|s| format!("{}", s))
                    .collect::<Vec<String>>()
                    .join("|");
                write!(f, "ShareSecrets from {}, secrets: {}", sender, repr)
            }
            Event::Mask {
                sender, random_id, ..
            } => write!(f, "Mask from {} for random: {}", sender, random_id),
            Event::Lock {
                sender, random_id, ..
            } => write!(f, "Lock from {} for random: {}", sender, random_id),
            Event::RandomnessReady { random_id } => {
                write!(f, "RandomnessReady, random_id: {}", random_id)
            }
            Event::Sync {
                new_players,
                new_servers,
                access_version,
                ..
            } => {
                let players = new_players
                    .iter()
                    .map(|p| p.addr.as_str())
                    .collect::<Vec<&str>>()
                    .join(",");
                let servers = new_servers
                    .iter()
                    .map(|s| s.addr.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ");

                write!(
                    f,
                    "Sync, new_players: [{}], new_servers: [{}], access_version = {}",
                    players, servers, access_version
                )
            }
            Event::Leave { player_addr } => write!(f, "Leave from {}", player_addr),
            Event::GameStart { access_version } => {
                write!(f, "GameStart, access_version = {}", access_version)
            }
            Event::WaitingTimeout => write!(f, "WaitTimeout"),
            Event::DrawRandomItems {
                sender,
                random_id,
                indexes,
            } => write!(
                f,
                "DrawRandomItems from {} for random {} with indexes {:?}",
                sender, random_id, indexes
            ),
            Event::DrawTimeout => write!(f, "DrawTimeout"),
            Event::ActionTimeout { player_addr } => write!(f, "ActionTimeout for {}", player_addr),
            Event::SecretsReady { random_ids } => {
                write!(f, "SecretsReady for {:?}", random_ids)
            }
            Event::ServerLeave {
                server_addr,
                transactor_addr,
            } => write!(
                f,
                "ServerLeave {}, current transactor: {}",
                server_addr, transactor_addr
            ),
            Event::AnswerDecision { decision_id, .. } => {
                write!(f, "AnswerDecision for {}", decision_id)
            }
            Event::OperationTimeout { addrs } => {
                write!(f, "OperationTimeout for {:?}", addrs)
            }
            Event::Shutdown => {
                write!(f, "Shutdown")
            }
        }
    }
}

impl Event {
    pub fn custom<S: Into<String>, E: CustomEvent>(sender: S, e: &E) -> Self {
        Self::Custom {
            sender: sender.into(),
            raw: e.try_to_vec().unwrap(),
        }
    }
}

pub trait CustomEvent: Sized + BorshSerialize + BorshDeserialize {
    fn try_parse(slice: &[u8]) -> Result<Self, HandleError> {
        Self::try_from_slice(slice).or(Err(HandleError::MalformedCustomEvent))
    }
}

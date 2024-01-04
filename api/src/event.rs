use crate::{
    error::HandleError,
    types::{Ciphertext, DecisionId, GamePlayer, RandomId, SecretDigest, SecretShare},
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
        sender: u64,
        raw: Vec<u8>,
    },

    /// A event sent by system, the first event sent by transactor
    /// when game is loaded.
    Ready,

    /// Transactor shares its secert to specific player.
    /// The `secret_data` is encrypted with the receiver's public key.
    ShareSecrets {
        sender: u64,
        shares: Vec<SecretShare>,
    },

    OperationTimeout {
        ids: Vec<u64>,
    },

    /// Randomize items.
    /// This event is sent by transactors.
    Mask {
        sender: u64,
        random_id: RandomId,
        ciphertexts: Vec<Ciphertext>,
    },

    /// Lock items.
    /// This event is sent by transactors.
    Lock {
        sender: u64,
        random_id: RandomId,
        ciphertexts_and_digests: Vec<(Ciphertext, SecretDigest)>,
    },

    /// All randomness is prepared.
    /// This event is sent by transactor.
    RandomnessReady {
        random_id: RandomId,
    },

    /// This event is sent when new players joined game.
    Join {
        players: Vec<GamePlayer>,
    },

    /// A server left the game.
    /// `transactor_addr` is the new current transactor address.
    ///
    /// NOTE: This event must be handled idempotently.
    ServerLeave {
        server_id: u64,
    },

    /// Client left game
    /// This event is sent by transactor based on client's connection status.
    Leave {
        player_id: u64,
    },

    /// Transactor uses this event as the start for each game.
    GameStart,

    /// Timeout when waiting for start
    WaitingTimeout,

    /// Random drawer takes random items by indexes.
    DrawRandomItems {
        sender: u64,
        random_id: usize,
        indexes: Vec<usize>,
    },

    /// Timeout for drawing random items
    DrawTimeout,

    /// Timeout when waiting for player's action
    /// Sent by transactor.
    ActionTimeout {
        player_id: u64,
    },

    /// Answer the decision question with encrypted ciphertext
    AnswerDecision {
        sender: u64,
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

    /// The custom event from bridge
    Bridge {
        dest: usize,
        raw: Vec<u8>,
    },
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Custom { sender, raw } => write!(f, "Custom from {}, inner: {:?}", sender, raw),
            Event::Bridge { dest, raw } => write!(f, "Bridge to {}, inner: {:?}", dest, raw),
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
            Event::Join { players } => {
                let players = players
                    .iter()
                    .map(|p| p.id.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "Join, players: [{}]", players)
            }
            Event::Leave { player_id } => write!(f, "Leave from {}", player_id),
            Event::GameStart { } => {
                write!(f, "GameStart")
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
            Event::ActionTimeout { player_id } => write!(f, "ActionTimeout for {}", player_id),
            Event::SecretsReady { random_ids } => {
                write!(f, "SecretsReady for {:?}", random_ids)
            }
            Event::ServerLeave {
                server_id,
            } => write!(
                f,
                "ServerLeave {}",
                server_id
            ),
            Event::AnswerDecision { decision_id, .. } => {
                write!(f, "AnswerDecision for {}", decision_id)
            }
            Event::OperationTimeout { ids } => {
                write!(f, "OperationTimeout for {:?}", ids)
            }
            Event::Shutdown => {
                write!(f, "Shutdown")
            }
        }
    }
}

impl Event {
    pub fn custom<E: CustomEvent>(sender: u64, e: &E) -> Self {
        Self::Custom {
            sender,
            raw: e.try_to_vec().unwrap(),
        }
    }

    pub fn bridge<E: BridgeEvent>(dest: usize, e: &E) -> Self {
        Self::Bridge {
            dest,
            raw: e.try_to_vec().unwrap(),
        }
    }
}

pub trait CustomEvent: Sized + BorshSerialize + BorshDeserialize {
    fn try_parse(slice: &[u8]) -> Result<Self, HandleError> {
        Self::try_from_slice(slice).or(Err(HandleError::MalformedCustomEvent))
    }
}

pub trait BridgeEvent: Sized + BorshSerialize + BorshDeserialize {
    fn try_parse(slice: &[u8]) -> Result<Self, HandleError> {
        Self::try_from_slice(slice).or(Err(HandleError::MalformedBridgeEvent))
    }
}

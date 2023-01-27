use crate::types::{Ciphertext, SecretDigest, SecretShare, RandomId};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum Event {
    /// Custom game event sent by players.
    Custom { sender: String, raw: String },

    /// Client marks itself as ready for the next game
    /// This event is sent by client.
    Ready { sender: String },

    /// Transactor shares its secert to specific player.
    /// The `secret_data` is encrypted with the receiver's public key.
    ShareSecrets {
        sender: String,
        secrets: Vec<SecretShare>,
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
    RandomnessReady,

    /// Client joined game.
    /// This event is sent by transactor based on client's connection status.
    ///
    /// NOTE: This event must be handled idempotently.
    Join {
        player_addr: String,
        balance: u64,
        position: usize,
    },

    /// New server attached to the game.  `transactor_addr` is the new
    /// current transactor address.
    ///
    /// NOTE: This event must be handled idempotently.
    ServerJoin {
        server_addr: String,
        endpoint: String,
        transactor_addr: String,
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
    Leave { player_addr: String },

    /// Transactor uses this game as the start for each game.
    GameStart,

    /// Timeout when waiting for start
    WaitTimeout,

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
    ActionTimeout { player_addr: String },

    /// All required secrets are shared
    SecretsReady,
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Custom { sender, raw } => write!(f, "Custom from {}, inner: {}", sender, raw),
            Event::Ready { sender } => write!(f, "Ready from {}", sender),
            Event::ShareSecrets { sender, .. } => write!(f, "ShareSecrets from {}", sender),
            Event::Mask {
                sender, random_id, ..
            } => write!(f, "Mask from {} for random {}", sender, random_id),
            Event::Lock {
                sender, random_id, ..
            } => write!(f, "Lock from {} for random: {}", sender, random_id),
            Event::RandomnessReady => write!(f, "RandomnessReady"),
            Event::Join {
                player_addr,
                balance,
                position,
            } => write!(
                f,
                "Join from {}, with balance: {}, position: {}",
                player_addr, balance, position
            ),
            Event::Leave { player_addr } => write!(f, "Leave from {}", player_addr),
            Event::GameStart => write!(f, "GameStart"),
            Event::WaitTimeout => write!(f, "WaitTimeout"),
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
            Event::ActionTimeout { player_addr } => write!(f, "ActionTimeout from {}", player_addr),
            Event::SecretsReady => write!(f, "SecretsReady"),
            Event::ServerJoin {
                server_addr,
                endpoint,
                transactor_addr,
            } => write!(
                f,
                "ServerJoin from {}, endpoint: {}, current transactor: {}",
                server_addr, endpoint, transactor_addr
            ),
            Event::ServerLeave {
                server_addr,
                transactor_addr,
            } => write!(
                f,
                "ServerLeave {}, current transactor: {}",
                server_addr, transactor_addr
            ),
        }
    }
}

impl Event {
    pub fn custom<S: Into<String>, E: CustomEvent>(sender: S, e: &E) -> Self {
        Self::Custom {
            sender: sender.into(),
            raw: serde_json::to_string(&e).unwrap(),
        }
    }
}

pub trait CustomEvent: Serialize + DeserializeOwned + Sized {}

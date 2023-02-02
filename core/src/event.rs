use crate::types::{Ciphertext, PlayerJoin, RandomId, SecretDigest, SecretShare, ServerJoin};
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
    RandomnessReady { random_id: RandomId },

    /// Sync with on-chain account.
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
    Leave { player_addr: String },

    /// Transactor uses this game as the start for each game.
    /// The `access_version` can be used to filter out which players are included.
    GameStart { access_version: u64 },

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
            Event::ShareSecrets { sender, secrets } => {
                let repr = secrets
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
                transactor_addr,
                access_version,
            } => write!(
                f,
                "Sync, new_players: {:?}, new_servers: {:?}, transactor: {}, access_version = {}",
                new_players, new_servers, transactor_addr, access_version
            ),
            Event::Leave { player_addr } => write!(f, "Leave from {}", player_addr),
            Event::GameStart { access_version } => {
                write!(f, "GameStart, access_version = {}", access_version)
            }
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

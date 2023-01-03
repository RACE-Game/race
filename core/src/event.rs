use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::types::{Ciphertext, SecretDigest};

#[derive(Hash, Debug, BorshDeserialize, BorshSerialize, PartialEq, Eq, Serialize, Deserialize, Clone, PartialOrd, Ord)]
pub struct SecretIdent {
    pub from_addr: String,
    pub to_addr: Option<String>,
    pub random_id: u32,
    pub index: u32,
}

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
        secret_ident: SecretIdent,
        secret_data: String,
    },

    /// Randomize items.
    /// This event is sent by transactors.
    Randomize {
        sender: String,
        random_id: usize,
        ciphertexts: Vec<Ciphertext>
    },

    /// Lock items.
    /// This event is sent by transactors.
    Lock {
        sender: String,
        random_id: usize,
        ciphertexts_and_digests: Vec<(Ciphertext, SecretDigest)>,
    },

    /// All randomness is prepared.
    /// This event is sent by transactor.
    RandomnessReady,

    /// Client joined game.
    /// This event is sent by transactor based on client's connection status.
    Join { player_addr: String, balance: u64 },

    /// Client left game
    /// This event is sent by transactor based on client's connection status.
    Leave { player_addr: String },

    /// Transactor uses this game as the start for each game.
    GameStart,

    /// Timeout when waiting for start
    WaitTimeout,

    /// Random drawer takes random items by indexes.
    DrawRandomItems { sender: String, random_id: usize, indexes: Vec<usize> },

    /// Timeout for drawing random items
    DrawTimeout,

    /// Timeout when waiting for player's action
    /// Sent by transactor.
    ActionTimeout { player_addr: String },

    /// All required secrets are shared
    SecretsReady,
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

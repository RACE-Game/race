use std::collections::HashMap;

use crate::types::{Ciphertext, SecretDigest};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(
    Hash,
    Debug,
    BorshDeserialize,
    BorshSerialize,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Clone,
    PartialOrd,
    Ord,
)]
pub struct SecretIdent {
    pub from_addr: String,
    pub to_addr: Option<String>,
    pub random_id: usize,
    pub index: usize,
}

#[derive(
    Hash,
    Debug,
    BorshDeserialize,
    BorshSerialize,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Clone,
    PartialOrd,
    Ord,
)]
pub struct ItemIdent {
    pub random_id: usize,
    pub index: usize,
}

impl SecretIdent {
    pub fn new_for_assigned<S: Into<String>>(
        random_id: usize,
        index: usize,
        from_addr: S,
        to_addr: S,
    ) -> Self {
        SecretIdent {
            from_addr: from_addr.into(),
            to_addr: Some(to_addr.into()),
            random_id,
            index,
        }
    }

    pub fn new_for_revealed<S: Into<String>>(random_id: usize, index: usize, from_addr: S) -> Self {
        SecretIdent {
            from_addr: from_addr.into(),
            to_addr: None,
            random_id,
            index,
        }
    }
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
        secrets: HashMap<SecretIdent, Vec<u8>>,
    },

    /// Randomize items.
    /// This event is sent by transactors.
    Mask {
        sender: String,
        random_id: usize,
        ciphertexts: Vec<Ciphertext>,
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
    Join { player_addr: String, balance: u64, position: usize },

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

impl Event {
    pub fn custom<S: Into<String>, E: CustomEvent>(sender: S, e: &E) -> Self {
        Self::Custom {
            sender: sender.into(),
            raw: serde_json::to_string(&e).unwrap(),
        }
    }
}

pub trait CustomEvent: Serialize + DeserializeOwned + Sized {}

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub struct SecretIdent {
    from_addr: String,
    to_addr: Option<String>,
    secret_key: String,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq, Eq)]
pub enum Event {
    /// Custom game events
    Custom(String),
    /// Client joined game
    Join { player_addr: String, timestamp: u64 },
    /// Client left game
    Leave { player_addr: String, timestamp: u64 },
    /// Client marks itself as ready for the next game
    Ready { player_addr: String, timestamp: u64 },
    /// Start the game
    GameStart { timestamp: u64 },
    /// Timeout when waiting for start
    WaitTimeout { timestamp: u64 },
    /// Timeout when waiting for player's action
    ActionTimeout { player_addr: String, timestamp: u64 },
    // /// Client shares its secret to others or public
    // ShareSecrets {
    //     player_addr: String,
    //     secret_ident: SecretIdent,
    //     secret_data: String,
    //     timestamp: u64,
    // },
    // /// Client requests secrets from others
    // RequestSecrets {
    //     player_addr: String,
    //     timestamp: u64,
    // },
    /// All required secrets are shared
    SecretsReady { timestamp: u64 },
    /// Some secrets didn't get shared
    // SecretMissing {
    //     missing_idents: Vec<SecretIdent>,
    //     timestamp: u64,
    // },
    /// All randomness is prepared
    RandomnessReady { timestamp: u64 },
}

impl Event {
    pub fn custom<E: CustomEvent>(e: &E) -> Self {
        Self::Custom(serde_json::to_string(&e).unwrap())
    }
}

pub trait CustomEvent: Serialize + DeserializeOwned {}

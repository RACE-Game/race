use thiserror::Error;
use serde::{Serialize, Deserialize};

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum Error {
    #[error("player already joined")]
    PlayerAlreadyJoined,

    #[error("player not in game")]
    PlayerNotInGame,

    #[error("malformed game event")]
    MalformedData(String),

    #[error("malformed address")]
    MalformedAddress,

    #[error("invalid randomness assignment")]
    InvalidRandomnessAssignment,

    #[error("invalid randomness revealing")]
    InvalidRandomnessRevealing,

    #[error("invalid random id")]
    InvalidRandomId,

    #[error("custom error")]
    Custom(String),

    #[error("game account not found")]
    GameAccountNotFound,

    #[error("game bundle not found")]
    GameBundleNotFound,

    #[error("rpc error")]
    RpcError(String),

    #[error("invalid chain name")]
    InvalidChainName,

    #[error("game not loaded")]
    GameNotLoaded,

    #[error("malformed game bundle")]
    MalformedGameBundle,
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::MalformedData(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

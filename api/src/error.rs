use borsh::{BorshDeserialize, BorshSerialize};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Error, Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HandleError {
    #[error("Custom error: {0}")]
    Custom(String),

    #[error("No enough players")]
    NoEnoughPlayers,

    #[error("No enough servers")]
    NoEnoughServers,

    #[error("Invalid player")]
    InvalidPlayer,

    #[error("Can't leave game")]
    CantLeave,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Malformed game account data")]
    MalformedGameAccountData,

    #[error("Malformed checkpoint data")]
    MalformedCheckpointData,

    #[error("Malformed custom event")]
    MalformedCustomEvent,

    #[error("Malformed bridge event")]
    MalformedBridgeEvent,

    #[error("Serialization error")]
    SerializationError,

    #[error("Cannot initialize subgame without checkpoint")]
    SubGameWithoutCheckpoint,

    #[error("Internal error: {message:?}")]
    InternalError { message: String },

    #[error("Invalid deposit")]
    InvalidDeposit,

    #[error("Duplicated bridge event target")]
    DuplicatedBridgeEventTarget,

    #[error("Randomness not revealed")]
    RandomnessNotRevealed,

    #[error("Answer not available")]
    AnswerNotAvailable,

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Invalid sub game id: {0}")]
    InvalidSubGameId(usize),
}

pub type HandleResult<T> = std::result::Result<T, HandleError>;

impl From<std::io::Error> for HandleError {
    fn from(e: std::io::Error) -> Self {
        HandleError::IoError(e.to_string())
    }
}

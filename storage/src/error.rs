use crate::arweave::error::Error as ArError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Arweave error: {0}")]
    ArweaveError(ArError),

    #[error("Unsupported chain: {0}")]
    UnsupportedChain(String),

    #[error("Name too long (should be less than 32)")]
    InvalidNameLength,

    #[error("Symbol too long (should be less than 10)")]
    InvalidSymbolLength,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<StorageError> for race_core::error::Error {
    fn from(value: StorageError) -> Self {
        race_core::error::Error::StorageError(value.to_string())
    }
}

use thiserror::Error;

use tokio::{task::JoinError, sync::mpsc::error::SendError, sync::oneshot::error::RecvError};

#[derive(Error, Debug)]
pub enum TransactorError {
    #[error("game not found")]
    GameNotFound,

    #[error("game already started")]
    GameAlreadyStarted,

    #[error("network error")]
    NetworkError(String),
}

impl From<JoinError> for TransactorError {
    fn from(e: JoinError) -> Self {
        println!("error: {:?}", e);
        TransactorError::NetworkError(e.to_string())
    }
}

impl From<RecvError> for TransactorError {
    fn from(e: RecvError) -> Self {
        println!("error: {:?}", e);
        TransactorError::NetworkError(e.to_string())
    }
}

impl<T> From<SendError<T>> for TransactorError {
    fn from(e: SendError<T>) -> Self {
        TransactorError::NetworkError(e.to_string())
    }
}

impl From<TransactorError> for jsonrpsee::core::Error {
    fn from(e: TransactorError) -> Self {
        jsonrpsee::core::Error::Custom(e.to_string())
    }
}

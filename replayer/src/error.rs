use std::io::{Error as IoError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReplayerError {
    #[error("Error: {0}")]
    RaceCoreError(race_core::error::Error),

    #[error("Io error: {0}")]
    IoError(IoError),

    #[error("Transport error: {0}")]
    TransportError(race_transport::error::TransportError),

    #[error("Missing header")]
    MissingHeader,

    #[error("RPC server error: {0}")]
    RpcError(jsonrpsee::core::error::Error),

    #[error("Addr parse error: {0}")]
    AddrParseError(std::net::AddrParseError),

    #[error("Replay not exists")]
    ReplayNotExists,
}

impl From<IoError> for ReplayerError {
    fn from(e: IoError) -> Self {
        ReplayerError::IoError(e)
    }
}

impl From<race_core::error::Error> for ReplayerError {
    fn from(e: race_core::error::Error) -> Self {
        ReplayerError::RaceCoreError(e)
    }
}

impl From<race_transport::error::TransportError> for ReplayerError {
    fn from(e: race_transport::error::TransportError) -> Self {
        ReplayerError::TransportError(e)
    }
}

impl From<jsonrpsee::core::error::Error> for ReplayerError {
    fn from(e: jsonrpsee::core::error::Error) -> Self {
        ReplayerError::RpcError(e)
    }
}

impl From<std::net::AddrParseError> for ReplayerError {
    fn from(e: std::net::AddrParseError) -> Self {
        ReplayerError::AddrParseError(e)
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("unspecified chain")]
    UnspecifiedChain,

    #[error("unspecified signer")]
    UnspecifiedSigner,

    #[error("unspecified rpc")]
    UnspecifiedRpc,

    #[error("invalid config")]
    InvalidConfig,

    #[error("invalid chain name")]
    InvalidChainName(String),

    #[error("initialization failed")]
    InitializationFailed(String),
}

pub type TransportResult<T> = std::result::Result<T, TransportError>;

impl From<TransportError> for race_core::error::Error {
    fn from(value: TransportError) -> Self {
        Self::InitializationTransportFailed(value.to_string())
    }
}

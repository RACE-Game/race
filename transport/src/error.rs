use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Unspecified chain")]
    UnspecifiedChain,

    #[error("Unspecified signer")]
    UnspecifiedSigner,

    #[error("Unspecified rpc")]
    UnspecifiedRpc,

    #[error("Invalid config")]
    InvalidConfig,

    #[error("Invalid chain name")]
    InvalidChainName(String),

    #[error("Initialization failed")]
    InitializationFailed(String),

    #[error("Invalid keyfile: {0}")]
    InvalidKeyfile(String),

    #[error("Lamports insufficient for rent exemption")]
    NoEnoughLamports,

    #[error("Failed to get lasted blockhash")]
    GetBlockhashFailed,

    #[error("Failed to send transaction from client")]
    ClientSendTransactionFailed,

    #[error("Client failed to get data from on chain account")]
    ClientGetDataFailed,
}

pub type TransportResult<T> = std::result::Result<T, TransportError>;

impl From<TransportError> for race_core::error::Error {
    fn from(value: TransportError) -> Self {
        Self::InitializationTransportFailed(value.to_string())
    }
}

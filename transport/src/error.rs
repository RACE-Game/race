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

    #[error("Invalid bundle address")]
    InvalidBundleAddress,

    #[error("Invalid chain name")]
    InvalidChainName(String),

    #[error("Initialization failed")]
    InitializationFailed(String),

    #[error("Failed to init the instruction")]
    InitInstructionFailed,

    #[error("Invalid keyfile: {0}")]
    InvalidKeyfile(String),

    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(String),

    #[error("Failed to get balance for pubkey: {0}")]
    InvalidBalance(String),

    #[error("Game account not found")]
    GameAccountNotFound,

    #[error("Failed to get game account data")]
    GameAccountDataNotFound,

    #[error("Player profile already exists")]
    DuplicatePlayerProfile,

    #[error("Failed to pack instruction data")]
    InstructionDataError,

    #[error("Lamports not enough for rent exemption")]
    NoEnoughLamports,

    #[error("Failed to create a pubkey")]
    PubkeyCreationFailed,

    #[error("Failed to get lasted blockhash")]
    GetBlockhashFailed,

    #[error("Failed to send transaction from client")]
    ClientSendTransactionFailed,

    #[error("Client failed to get data from on chain account")]
    ClientGetDataFailed,

    #[error("Failed to deserialize game account data")]
    GameStateDeserializeError,

    #[error("Failed to parse string address")]
    ParseAddressError,
}

pub type TransportResult<T> = std::result::Result<T, TransportError>;

impl From<TransportError> for race_core::error::Error {
    fn from(value: TransportError) -> Self {
        Self::InitializationTransportFailed(value.to_string())
    }
}

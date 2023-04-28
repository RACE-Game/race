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

    #[error("Invalid program id")]
    InvalidProgramID,

    #[error("Invalid chain name: {0}")]
    InvalidChainName(String),

    #[error("Game title or nick name exceeds 16 characters: {0}")]
    InvalidNameLength(String),

    #[error("Initialization failed: {0}")]
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

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Failed to get game account data")]
    GameAccountDataNotFound,

    #[error("Failed to get server account data")]
    ServerAccountDataNotFound,

    #[error("Failed to get player account data")]
    PlayerAccountDataNotFound,

    #[error("Endpoint length exceeds 50 chars")]
    EndpointTooLong,

    #[error("Failed to get registry account data")]
    RegistryAccountDataNotFound,

    #[error("Player profile already exists")]
    DuplicatePlayerProfile,

    #[error("Server account already exists")]
    DuplicateServerAccount,

    #[error("Failed to pack instruction data")]
    InstructionDataError,

    #[error("Lamports not enough for rent exemption")]
    NoEnoughLamports,

    #[error("Failed to create an adress")]
    AddressCreationFailed,

    #[error("Failed to get lasted blockhash")]
    GetBlockhashFailed,

    #[error("Failed to send transaction from client: {0}")]
    ClientSendTransactionFailed(String),

    #[error("Client failed to get data from on chain account")]
    ClientGetDataFailed,

    #[error("Failed to deserialize game account data")]
    GameStateDeserializeError,

    #[error("Failed to deserialize server account data")]
    ServerStateDeserializeError,

    #[error("Failed to deserialize player account data")]
    PlayerStateDeserializeError,

    #[error("Failed to deserialize registry account data")]
    RegistryStateDeserializeError,

    #[error("Failed to deserialize metadata account data")]
    MetadataDeserializeError,

    #[error("Failed to parse string address")]
    ParseAddressError,

    #[error("Transaction is not confirmed")]
    TransactionNotConfirmed,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Duplicated vote")]
    DuplicatedVote,

    #[error("InteropError")]
    InteropError,

    #[error("Failed to create instruction: {0}")]
    InstructionCreationError(String),

    #[error("Access versions not matched")]
    AccessVersionNotMatched,

    #[error("Failed to mint NFT: {0}")]
    MintNFTError(String),

    #[error("Failed to load game bundle: {0}")]
    LoadGameBundleError(String),

    #[error("Invalid parameter")]
    InvalidParameter(String)
}

pub type TransportResult<T> = std::result::Result<T, TransportError>;

impl From<TransportError> for race_core::error::Error {
    fn from(value: TransportError) -> Self {
        Self::TransportError(value.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<race_nft_storage::Error> for TransportError {
    fn from(value: race_nft_storage::Error) -> Self {
        Self::LoadGameBundleError(value.to_string())
    }
}

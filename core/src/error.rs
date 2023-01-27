use thiserror::Error;
use serde::{Serialize, Deserialize};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Error, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("Player already joined")]
    PlayerAlreadyJoined,

    #[error("Player not in game")]
    PlayerNotInGame,

    #[error("Malformed game event")]
    MalformedData(String),

    #[error("Malformed address")]
    MalformedAddress,

    #[error("Invalid randomness assignment")]
    InvalidRandomnessAssignment,

    #[error("Invalid randomness revealing")]
    InvalidRandomnessRevealing,

    #[error("Invalid random id")]
    InvalidRandomId,

    #[error("Custom error")]
    Custom(String),

    #[error("Game account not found")]
    GameAccountNotFound,

    #[error("Game bundle not found")]
    GameBundleNotFound,

    #[error("Server account exists")]
    ServerAccountExists,

    #[error("Rpc error: {0}")]
    RpcError(String),

    #[error("Invalid chain name")]
    InvalidChainName,

    #[error("Invalid player address")]
    InvalidPlayerAddress,

    #[error("Invalid player status")]
    InvalidPlayerStatus,

    #[error("Game not loaded")]
    GameNotLoaded,

    #[error("Malformed endpoint")]
    MalformedEndpoint,

    #[error("Malformed game bundle")]
    MalformedGameBundle,

    #[error("Malformed game account")]
    MalformedGameAccount,

    #[error("Deserialize error")]
    DeserializeError,

    #[error("Config missing")]
    ConfigMissing,

    #[error("Transactor config missing")]
    TransactorConfigMissing,

    #[error("Can't leave")]
    CantLeave,

    #[error("Randomization error: {0}")]
    RandomizationError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Duplicated event dispatching")]
    DuplicatedEventDispatching,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Not allowed in custom handler")]
    NotAllowedInCustomHandler,

    #[error("Game not served")]
    GameNotServed,

    #[error("Game is not empty")]
    GameIsNotEmpty,

    #[error("Can't find transactor")]
    CantFindTransactor,

    #[error("Invalid transactor address")]
    InvalidTransactorAddress,

    #[error("Failed to load on-chain server account, please register first")]
    ServerAccountMissing,

    #[error("Initialization transport failed: {0}")]
    InitializationTransportFailed(String),

    #[error("Initialize rpc client error: {0}")]
    InitializeRpcClientError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Missing secret")]
    MissingSecret,

    #[error("Invalid secret")]
    InvalidSecret,

    #[error("Invalid decrypted value: {0}")]
    InvalidDecryptedValue(String),

    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Invalid key index")]
    InvalidKeyIndex,

    #[error("Invalid ciphertexts size")]
    InvalidCiphertextsSize,

    #[error("Invalid max players")]
    InvalidMaxPlayers,

    #[error("JSON parse error")]
    JsonParseError,

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid Settle")]
    InvalidSettle,

    #[error("IO Error: {0}")]
    IoError(String),

    #[error("Not supported in validator mode")]
    NotSupportedInValidatorMode,
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::MalformedData(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

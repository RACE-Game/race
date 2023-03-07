use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::DecisionId;

#[derive(
    Error, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq,
)]
pub enum Error {
    #[error("Player already joined: {0}")]
    PlayerAlreadyJoined(String),

    #[error("Game is full: {0}")]
    GameIsFull(u32),

    #[error("Server queue is full: {0}")]
    ServerQueueIsFull(u32),

    #[error("No enough players")]
    NoEnoughPlayers,

    #[error("Server already joined: {0}")]
    ServerAlreadyJoined(String),

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
    ServerAccountNotFound,

    #[error("Player profile not found")]
    PlayerProfileNotFound,

    #[error("Game account not found")]
    GameAccountNotFound,

    #[error("Game bundle not found")]
    GameBundleNotFound,

    #[error("Server account exists")]
    ServerAccountExists,

    #[error("Registration not found")]
    RegistrationNotFound,

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

    #[error("Invalid voter: {0}")]
    InvalidVoter(String),

    #[error("Invalid votee: {0}")]
    InvalidVotee(String),

    #[error("Duplicated vote")]
    DuplicatedVote,

    #[error("Transaction expired")]
    TransactionExpired,

    #[error("Event ignored")]
    EventIgnored,

    #[error("Invalid custom event")]
    InvalidCustomEvent,

    #[error("Invalid decision id")]
    InvalidDecisionId,

    #[error("Answer not available")]
    AnswerNotAvailable,

    #[error("Missing decision secret: {0}")]
    MissingDecisionSecret(DecisionId),

    #[error("Invalid decision answer")]
    InvalidDecisionAnswer,

    #[error("Invalid decision owner")]
    InvalidDecisionOwner,

    #[error("Invalid decision status")]
    InvalidDecisionStatus,

    #[error("Duplicated secret share")]
    DuplicatedSecretShare,

    #[error("Serialization error")]
    SerializationError,

    #[error("Wasm execution error: {0}")]
    WasmExecutionError(String),

    #[error("Wasm memory overflow")]
    WasmMemoryOverflow,

    #[error("Invalid checkpoint")]
    InvalidCheckpoint,

    #[error("Duplicated initialization")]
    DuplicatedInitialization,
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

use borsh::{BorshDeserialize, BorshSerialize};
use race_api::error::HandleError;

use anyhow;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Error {
    #[error("Player already joined: {0}")]
    PlayerAlreadyJoined(String),

    #[error("Position Occupied: {0}")]
    PositionOccupied(usize),

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

    #[error("Server account not found")]
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

    #[error("Recipient not found on chain")]
    RecipientNotFound,

    #[error("Recipient slot not found on chain")]
    RecipientSlotNotFound,

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

    #[error("Invalid handler state")]
    InvalidHandlerState,

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
    TransportError(String),

    #[error("Initializing instruction failed: {0}")]
    InitInstructionFailed(String),

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

    #[error("Invalid ciphertexts size, expect: {0}, got: {1}")]
    InvalidCiphertextsSize(u32, u32),

    #[error("Invalid max players")]
    InvalidMaxPlayers,

    #[error("JSON parse error")]
    JsonParseError,

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid Settle: {0}")]
    InvalidSettle(String),

    #[error("IO Error: {0}")]
    IoError(String),

    #[error("Not supported in validator mode")]
    NotSupportedInValidatorMode,

    #[error("Not supported in sub game mode")]
    NotSupportedInSubGameMode,

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

    #[error("Wallet not connected")]
    WalletNotConnected,

    #[error("Invalid custom event")]
    InvalidCustomEvent,

    #[error("Invalid decision id")]
    InvalidDecisionId,

    #[error("Answer not available")]
    AnswerNotAvailable,

    #[error("Missing decision secret: {0}")]
    MissingDecisionSecret(usize),

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

    #[error("Wasm initialization error: {0}")]
    WasmInitializationError(String),

    #[error("Wasm execution error: {0}")]
    WasmExecutionError(String),

    #[error("Wasm memory overflow")]
    WasmMemoryOverflow,

    #[error("Invalid checkpoint")]
    InvalidCheckpoint,

    #[error("Duplicated initialization")]
    DuplicatedInitialization,

    #[error("Randomness is not revealed")]
    RandomnessNotRevealed,

    #[error("Random state not found: {0}")]
    RandomStateNotFound(usize),

    #[error("Wasm execution error: {0}")]
    HandleError(HandleError),

    #[error("Invalid recipient slot params")]
    InvalidRecipientSlotParams,

    #[error("Error in storage interaction: {0}")]
    StorageError(String),

    #[error("Cannot settle or transfer without checkpoint")]
    SettleWithoutCheckpoint,

    #[error("Settle version mismatch, given: {0}, expected: {1}")]
    SettleVersionMismatch(u64, u64),

    #[error("Duplicated sub game id")]
    DuplicatedSubGameId,

    #[error("Cannot map id to address: {0}")]
    CantMapIdToAddr(u64),

    #[error("Cannot map address to id: {0}")]
    CantMapAddrToId(String),

    #[error("Cannot bump settle version")]
    CantBumpSettleVersion,

    #[error("Game uninitialized")]
    GameUninitialized,

    #[error("Game closed")]
    GameClosed,

    #[error("Invalid bridge event")]
    InvalidBridgeEvent,

    #[error("Invalid player id found when {2}, id: {0}, availables: {1:?}")]
    InvalidPlayerId(u64, Vec<u64>, String),

    #[error("Invalid game id")]
    InvalidGameId,

    #[error("Invalid sub game id")]
    InvalidSubGameId,

    #[error("Duplicated bridge event target")]
    DuplicatedBridgeEventTarget,

    #[error("Missing init data")]
    MissingInitData,

    #[error("Checkpoint state sha mismatch")]
    CheckpointStateShaMismatch,

    #[error("Malformed checkpoint")]
    MalformedCheckpoint,

    #[error("Add balance error, player: {0}, origin: {1}, amount: {2}")]
    AddBalanceError(u64, u64, u64), // player_id, from_balance, to_balance

    #[error("Sub balance error, player: {0}, origin: {1}, amount: {2}")]
    SubBalanceError(u64, u64, u64), // player_id, from_balance, to_balance

    #[error("Missing Checkpoint")]
    MissingCheckpoint,

    #[error("Checkpoint already exists")]
    CheckpointAlreadyExists,

    #[error("Checkpoint not found after initialization")]
    CheckpointNotFoundAfterInit,

    #[error("Handled deposit rejection")]
    HandledDepositRejection,

    #[error("Empty reject deposits")]
    EmptyRejectDeposits,

    #[error("Math overflow")]
    MathOverflow,
}

#[cfg(feature = "serde")]
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


impl From<crate::error::Error> for HandleError {
    fn from(value: crate::error::Error) -> Self {
        HandleError::InternalError {
            message: value.to_string(),
        }
    }
}

impl From<HandleError> for Error {
    fn from(value: HandleError) -> Self {
        Error::HandleError(value)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::TransportError(e.to_string())
    }
}

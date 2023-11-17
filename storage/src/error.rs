use base64::DecodeError;
use serde_json;
use reqwest::Error as HttpError;
use std::io::Error as IoError;
use thiserror::Error;
use openssl;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error getting arweave price: {0}")]
    ArweaveGetPriceError(reqwest::Error),

    #[error("error posting arweave transaction: {0}")]
    ArweavePostError(reqwest::Error),

    #[error("error get arweave last tx: {0}")]
    ArweaveLastTxError(reqwest::Error),

    #[error("error get wallet balance: {0}")]
    ArweaveWalletBalanceError(reqwest::Error),

    #[error("base64 decode: {0}")]
    Base64Decode(#[from] DecodeError),

    #[error("failed to deserialize branch proof")]
    DerserializeBranchProofError,

    #[error("failed to get Base64 type from utf8: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[error("failed to convert Base64 to string: {0}")]
    ToStringError(#[from] std::fmt::Error),

    #[error("hashing failed")]
    InvalidHash,

    #[error("invalid proof")]
    InvalidProof,

    #[error("invalid winston amount")]
    InvalidWinstonAmount(#[from] std::num::ParseIntError),

    #[error("name too long (should be less than 32)")]
    InvalidNameLength,

    #[error("symbol too long (should be less than 10)")]
    InvalidSymbolLength,

    #[error("io error: {0}")]
    IOError(#[from] IoError),
    // #[error("keypair not provided")]
    // KeyPairNotProvided,
    // #[error("key rejected: {0}")]
    // KeyRejected(#[from] KeyRejected),
    // #[error("manifest not found")]
    // ManifestNotFound,
    // #[error("file path not provided")]
    // MissingFilePath,
    // #[error("missing trailing slash")]
    #[error("no root node found")]
    NoRootNodeFound,

    #[error("openssl error: {0}")]
    OpenSSLError(#[from] openssl::error::ErrorStack),

    #[error("reqwest: {0}")]
    Reqwest(#[from] HttpError),

    // #[error("ring rsa key pair rejected from jwk components")]
    // RingRSAKeyPairRejected(#[from] ring::error::KeyRejected),

    #[error("serde json: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("post response status code not ok")]
    StatusCodeNotOk,

    // #[error("status not found")]
    // StatusNotFound,
    // #[error("solana hash parse {0}")]
    // SolanaHashParse(#[from] solana_sdk::hash::ParseHashError),
    // #[error("solana network error")]
    // SolanaNetworkError,
    // #[error("solana hash parse {0}")]
    // TokioJoinError(#[from] tokio::task::JoinError),
    #[error("transaction is not signed")]
    UnsignedTransaction,
}

pub type Result<T> = std::result::Result<T, Error>;

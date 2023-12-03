use base64::DecodeError;
use serde_json;
use reqwest::Error as HttpError;
use std::io::Error as IoError;
use thiserror::Error;
use openssl;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error getting arweave price: {0}")]
    ArweaveGetPriceError(reqwest::Error),

    #[error("Error posting arweave transaction: {0}")]
    ArweavePostError(reqwest::Error),

    #[error("Error get arweave last tx: {0}")]
    ArweaveLastTxError(reqwest::Error),

    #[error("Error get wallet balance: {0}")]
    ArweaveWalletBalanceError(reqwest::Error),

    #[error("Base64 decode: {0}")]
    Base64Decode(#[from] DecodeError),

    #[error("Failed to deserialize branch proof")]
    DerserializeBranchProofError,

    #[error("Failed to get Base64 type from utf8: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[error("Failed to convert Base64 to string: {0}")]
    ToStringError(#[from] std::fmt::Error),

    #[error("Hashing failed")]
    InvalidHash,

    #[error("Invalid proof")]
    InvalidProof,

    #[error("Invalid winston amount")]
    InvalidWinstonAmount(#[from] std::num::ParseIntError),

    #[error("Io error: {0}")]
    IOError(#[from] IoError),

    #[error("No root node found")]
    NoRootNodeFound,

    #[error("Openssl error: {0}")]
    OpenSSLError(#[from] openssl::error::ErrorStack),

    #[error("Reqwest: {0}")]
    Reqwest(#[from] HttpError),

    #[error("Serde json: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Post response status code not ok")]
    StatusCodeNotOk,

    #[error("Transaction is not signed")]
    UnsignedTransaction,
}

pub type Result<T> = std::result::Result<T, Error>;

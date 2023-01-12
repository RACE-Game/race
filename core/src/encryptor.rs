use crate::types::{SecretKey, Ciphertext, SecretDigest};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("key gen failed")]
    KeyGenFailed,

    #[error("encode failed")]
    EncodeFailed,

    #[error("rsa encrypt failed")]
    RsaEncryptFailed(String),

    #[error("rsa decrypt failed")]
    RsaDecryptFailed(String),

    #[error("sign failed")]
    SignFailed(String),

    #[error("verify failed")]
    VerifyFailed(String),

    #[error("aes encrypt failed")]
    AesEncryptFailed,

    #[error("aes decrypt failed")]
    AesDecryptFailed,

    #[error("public key not found")]
    PublicKeyNotfound,

    #[error("failed to import public key")]
    ImportPublicKeyError,

    #[error("failed to import private key")]
    ImportPrivateKeyError,
}

impl From<Error> for crate::error::Error {
    fn from(e: Error) -> Self {
        crate::error::Error::CryptoError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait EncryptorT: std::fmt::Debug + Send + Sync {

    fn add_public_key(&mut self, addr: String, raw: &str) -> Result<()>;

    fn export_public_key(&self, addr: Option<&str>) -> Result<String>;

    fn gen_secret(&self) -> SecretKey;

    fn encrypt(&self, addr: Option<&str>, text: &[u8]) -> Result<Vec<u8>>;

    fn decrypt(&self, text: &[u8]) -> Result<Vec<u8>>;

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;

    fn apply(&self, secret: &SecretKey, buf: &mut [u8]);

    fn apply_multi(&self, secret: Vec<SecretKey>, buf: &mut [u8]);

    fn verify(&self, addr: Option<&str>, message: &[u8], signature: &[u8]) ->  Result<()>;

    fn shuffle(&self, items: &mut Vec<Ciphertext>);

    fn digest(&self, text: &[u8]) -> SecretDigest;
}

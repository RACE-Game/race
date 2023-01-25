use std::collections::HashMap;

use crate::types::{Ciphertext, SecretDigest, SecretKey, Signature};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Key gen failed")]
    KeyGenFailed,

    #[error("Encode failed")]
    EncodeFailed,

    #[error("Decode failed")]
    DecodeFailed,

    #[error("Rsa encrypt failed")]
    RsaEncryptFailed(String),

    #[error("Rsa decrypt failed")]
    RsaDecryptFailed(String),

    #[error("Sign failed: {0}")]
    SignFailed(String),

    #[error("Invalid result: {0}")]
    InvalidResult(String),

    #[error("Verify failed: {0}")]
    VerifyFailed(String),

    #[error("Aes encrypt failed")]
    AesEncryptFailed,

    #[error("Aes decrypt failed")]
    AesDecryptFailed,

    #[error("Public key not found")]
    PublicKeyNotfound,

    #[error("Failed to import public key")]
    ImportPublicKeyError,

    #[error("Failed to export public key")]
    ExportPublicKeyError,

    #[error("Failed to import private key")]
    ImportPrivateKeyError,

    #[error("Invalid nonce")]
    InvalidNonce,

    #[error("Add public key error")]
    AddPublicKeyError,

    #[error("Read public key error")]
    ReadPublicKeyError,

    #[error("Missing secrets")]
    MissingSecret,
}

impl From<Error> for crate::error::Error {
    fn from(e: Error) -> Self {
        crate::error::Error::CryptoError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait EncryptorT: std::fmt::Debug + Send + Sync {
    fn add_public_key(&self, addr: String, raw: &str) -> Result<()>;

    fn export_public_key(&self, addr: Option<&str>) -> Result<String>;

    fn gen_secret(&self) -> SecretKey;

    fn encrypt(&self, addr: Option<&str>, text: &[u8]) -> Result<Vec<u8>>;

    fn decrypt(&self, text: &[u8]) -> Result<Vec<u8>>;

    fn apply(&self, secret: &SecretKey, buf: &mut [u8]);

    fn apply_multi(&self, secret: Vec<SecretKey>, buf: &mut [u8]);

    fn sign_raw(&self, message: &[u8]) -> Result<Vec<u8>>;

    fn verify_raw(&self, addr: Option<&str>, message: &[u8], signature: &[u8]) -> Result<()>;

    fn sign(&self, message: &[u8], signer: String) -> Result<Signature>;

    fn verify(&self, message: &[u8], signature: &Signature) -> Result<()>;

    fn shuffle(&self, items: &mut Vec<Ciphertext>);

    fn digest(&self, text: &[u8]) -> SecretDigest;

    fn decrypt_with_secrets(
        &self,
        ciphertext_map: HashMap<usize, Ciphertext>,
        mut secret_map: HashMap<usize, Vec<SecretKey>>,
        valid_options: &[String],
    ) -> Result<HashMap<usize, String>> {
        let mut ret = HashMap::new();
        for (i, mut buf) in ciphertext_map.into_iter() {
            if let Some(secrets) = secret_map.remove(&i) {
                self.apply_multi(secrets, &mut buf);
                let value = String::from_utf8(buf).or(Err(Error::DecodeFailed))?;
                if !valid_options.contains(&value) {
                    return Err(Error::InvalidResult(value))?;
                }
                ret.insert(i, value);
            } else {
                return Err(Error::MissingSecret);
            }
        }
        Ok(ret)
    }
}

pub trait Digestable {
    fn digest(&self) -> String;
}

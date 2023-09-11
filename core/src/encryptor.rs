use std::collections::HashMap;

use crate::types::{Ciphertext, SecretDigest, SecretKey, Signature};
use borsh::{BorshSerialize, BorshDeserialize};
use thiserror::Error;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum EncryptorError {
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

    #[error("Failed to export private key")]
    ExportPrivateKeyError,

    #[error("Invalid nonce")]
    InvalidNonce,

    #[error("Add public key error")]
    AddPublicKeyError,

    #[error("Read public key error")]
    ReadPublicKeyError,

    #[error("Missing secrets")]
    MissingSecret,

    #[error("Invalid signature length: {0}")]
    InvalidSignatureLength(usize),
}

impl From<EncryptorError> for crate::error::Error {
    fn from(e: EncryptorError) -> Self {
        crate::error::Error::CryptoError(e.to_string())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodePublicKeyRaw {
    pub rsa: String,
    pub ec: String,
}

pub type EncryptorResult<T> = std::result::Result<T, EncryptorError>;

pub trait EncryptorT: std::fmt::Debug + Send + Sync {
    fn add_public_key(&self, addr: String, raw: &NodePublicKeyRaw) -> EncryptorResult<()>;

    fn export_public_key(&self, addr: Option<&str>) -> EncryptorResult<NodePublicKeyRaw>;

    fn gen_secret(&self) -> SecretKey;

    fn encrypt(&self, addr: Option<&str>, text: &[u8]) -> EncryptorResult<Vec<u8>>;

    fn decrypt(&self, text: &[u8]) -> EncryptorResult<Vec<u8>>;

    fn apply(&self, secret: &SecretKey, buf: &mut [u8]);

    fn apply_multi(&self, secret: Vec<SecretKey>, buf: &mut [u8]);

    fn sign_raw(&self, message: &[u8]) -> EncryptorResult<Vec<u8>>;

    fn verify_raw(
        &self,
        addr: Option<&str>,
        message: &[u8],
        signature: &[u8],
    ) -> EncryptorResult<()>;

    fn sign(&self, message: &[u8], signer: String) -> EncryptorResult<Signature>;

    fn verify(&self, message: &[u8], signature: &Signature) -> EncryptorResult<()>;

    fn shuffle(&self, items: &mut Vec<Ciphertext>);

    fn digest(&self, text: &[u8]) -> SecretDigest;

    fn decrypt_with_secrets(
        &self,
        ciphertext_map: HashMap<usize, Ciphertext>,
        mut secret_map: HashMap<usize, Vec<SecretKey>>,
        valid_options: &[String],
    ) -> EncryptorResult<HashMap<usize, String>> {
        let mut ret = HashMap::new();
        for (i, mut buf) in ciphertext_map.into_iter() {
            if let Some(secrets) = secret_map.remove(&i) {
                self.apply_multi(secrets, &mut buf);
                let value = String::from_utf8(buf).or(Err(EncryptorError::DecodeFailed))?;
                if !valid_options.contains(&value) {
                    return Err(EncryptorError::InvalidResult(value))?;
                }
                ret.insert(i, value);
            } else {
                return Err(EncryptorError::MissingSecret);
            }
        }
        Ok(ret)
    }
}

pub trait Digestable {
    fn digest(&self) -> String;
}

#[cfg(test)]
pub mod tests {
    use crate::types::{Ciphertext, SecretDigest, SecretKey, Signature};

    use super::{EncryptorResult, EncryptorT, NodePublicKeyRaw};

    #[derive(Debug, Default)]
    pub struct DummyEncryptor {}

    #[allow(unused)]
    impl EncryptorT for DummyEncryptor {
        fn add_public_key(&self, addr: String, raw: &NodePublicKeyRaw) -> EncryptorResult<()> {
            Ok(())
        }

        fn export_public_key(&self, addr: Option<&str>) -> EncryptorResult<NodePublicKeyRaw> {
            Ok(NodePublicKeyRaw {
                rsa: "".into(),
                ec: "".into(),
            })
        }

        fn gen_secret(&self) -> SecretKey {
            vec![0, 0, 0, 0]
        }

        fn encrypt(&self, addr: Option<&str>, text: &[u8]) -> EncryptorResult<Vec<u8>> {
            Ok(vec![0, 0, 0, 0])
        }

        fn decrypt(&self, text: &[u8]) -> EncryptorResult<Vec<u8>> {
            Ok(vec![0, 0, 0, 0])
        }

        fn apply(&self, secret: &SecretKey, buf: &mut [u8]) {}

        fn apply_multi(&self, secret: Vec<SecretKey>, buf: &mut [u8]) {}

        fn sign_raw(&self, message: &[u8]) -> EncryptorResult<Vec<u8>> {
            Ok(vec![0, 0, 0, 0])
        }

        fn verify_raw(
            &self,
            addr: Option<&str>,
            message: &[u8],
            signature: &[u8],
        ) -> EncryptorResult<()> {
            Ok(())
        }

        fn sign(&self, message: &[u8], signer: String) -> EncryptorResult<Signature> {
            Ok(Signature {
                signer,
                timestamp: 0,
                signature: "".into(),
            })
        }

        fn verify(&self, message: &[u8], signature: &Signature) -> EncryptorResult<()> {
            Ok(())
        }

        fn shuffle(&self, items: &mut Vec<Ciphertext>) {}

        fn digest(&self, text: &[u8]) -> SecretDigest {
            vec![0, 1, 2, 3]
        }
    }
}

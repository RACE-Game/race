//! We used an enhanced 2-role based mental poker algorithmn among a few nodes.
//! Each node can either be a player or a transactor.
//! For each node, there are two modes for randomization:
//! 1. Shuffler: participate in the shuffling, hold the secrets
//! 2. Drawer: pick the random item by index

use std::collections::HashMap;
use std::sync::Mutex;

use arrayref::{array_ref, array_refs, mut_array_refs};
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha20;
use race_core::encryptor::EncryptorT;
use race_core::types::Signature;
use rand::seq::SliceRandom;
use rsa::pkcs1::{FromRsaPrivateKey, FromRsaPublicKey, ToRsaPublicKey};
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use sha1::{Digest, Sha1};

use race_core::{
    encryptor::{Error, Result},
    types::{Ciphertext, SecretDigest, SecretKey},
};
use tracing::info;

#[derive(Debug)]
pub struct Encryptor {
    private_key: RsaPrivateKey,
    public_keys: Mutex<HashMap<String, RsaPublicKey>>,
    default_public_key: RsaPublicKey,
}

impl Encryptor {
    pub fn new(private_key: RsaPrivateKey) -> Self {
        let default_public_key = RsaPublicKey::from(&private_key);
        info!(
            "Encryptor created, public key: {}",
            hex::encode(default_public_key.to_pkcs1_der().unwrap().as_der(),)
        );
        Self {
            private_key,
            default_public_key,
            public_keys: Mutex::new(HashMap::new()),
        }
    }

    pub fn from_pem(pem: &str) -> Result<Self> {
        let private_key =
            RsaPrivateKey::from_pkcs1_pem(pem).or(Err(Error::ImportPrivateKeyError))?;
        Ok(Encryptor::new(private_key))
    }
}

impl Default for Encryptor {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let bits = 2048;
        let private_key = RsaPrivateKey::new(&mut rng, bits).expect("key gen failed");
        Encryptor::new(private_key)
    }
}

impl EncryptorT for Encryptor {
    fn gen_secret(&self) -> SecretKey {
        let mut secret = [0u8; 44];
        let (key, nonce) = mut_array_refs![&mut secret, 32, 12];
        key.copy_from_slice(&rand::random::<[u8; 32]>());
        nonce.copy_from_slice(&rand::random::<[u8; 12]>());
        secret.to_vec()
    }

    /// Encrypt the message use RSA public key
    fn encrypt(&self, addr: Option<&str>, text: &[u8]) -> Result<Vec<u8>> {
        let public_keys = self
            .public_keys
            .lock()
            .map_err(|_| Error::ReadPublicKeyError)?;
        let pubkey = match addr {
            Some(addr) => public_keys.get(addr).ok_or(Error::PublicKeyNotfound)?,
            None => &self.default_public_key,
        };
        let mut rng = rand::thread_rng();
        pubkey
            .encrypt(&mut rng, PaddingScheme::PKCS1v15Encrypt, text)
            .map_err(|e| Error::RsaEncryptFailed(e.to_string()))
    }

    /// Decrypt the message use RSA private key
    fn decrypt(&self, text: &[u8]) -> Result<Vec<u8>> {
        self.private_key
            .decrypt(PaddingScheme::PKCS1v15Encrypt, text)
            .map_err(|e| Error::RsaDecryptFailed(e.to_string()))
    }

    fn sign_raw(&self, message: &[u8]) -> Result<Vec<u8>> {
        let padding = PaddingScheme::new_pkcs1v15_sign(Some(rsa::Hash::SHA1));
        let hashed = Sha1::digest(message);
        // info!(
        //     "Verify signature, key: {:?}, message: {:?}",
        //     self.default_public_key, message
        // );
        self.private_key
            .sign(padding, &hashed)
            .map_err(|e| Error::SignFailed(e.to_string()))
    }

    fn sign(&self, message: &[u8], signer: String) -> Result<Signature> {
        // let timestamp = std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let timestamp = chrono::Utc::now().timestamp_millis() as _;
        let nonce: [u8; 8] = rand::random();
        let message = [message, &nonce, &u64::to_le_bytes(timestamp)].concat();
        let sig = self.sign_raw(&message)?;
        Ok(Signature {
            signer,
            nonce: hex::encode(nonce),
            timestamp: timestamp as _,
            signature: hex::encode(sig),
        })
    }

    fn verify_raw(&self, addr: Option<&str>, message: &[u8], signature: &[u8]) -> Result<()> {
        let public_keys = self
            .public_keys
            .lock()
            .map_err(|_| Error::ReadPublicKeyError)?;

        let pubkey = match addr {
            Some(addr) => public_keys.get(addr).ok_or(Error::PublicKeyNotfound)?,
            None => &self.default_public_key,
        };

        let padding = PaddingScheme::new_pkcs1v15_sign(Some(rsa::Hash::SHA1));
        let hashed = Sha1::digest(message).to_vec();
        pubkey
            .verify(padding, &hashed, signature)
            .map_err(|e| Error::VerifyFailed(e.to_string()))
    }

    fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        let Signature {
            signer,
            nonce,
            timestamp,
            signature,
        } = signature;
        // TODO: We should check timestamp here.
        let nonce = hex::decode(nonce).or(Err(Error::InvalidNonce))?;
        let signature = hex::decode(signature).or(Err(Error::InvalidNonce))?;
        let message = [message, &nonce, &u64::to_le_bytes(*timestamp)].concat();
        self.verify_raw(Some(&signer), &message, &signature)
    }

    fn apply(&self, secret: &SecretKey, buffer: &mut [u8]) {
        let secret = array_ref![secret, 0, 44];
        let (key, nonce) = array_refs![secret, 32, 12];
        let mut cipher = ChaCha20::new(key.into(), nonce.into());
        cipher.apply_keystream(buffer);
    }

    fn apply_multi(&self, secrets: Vec<SecretKey>, buffer: &mut [u8]) {
        for secret in secrets.into_iter() {
            self.apply(secret.as_ref(), buffer);
        }
    }

    fn shuffle(&self, items: &mut Vec<Ciphertext>) {
        let mut rng = rand::thread_rng();
        items.shuffle(&mut rng);
    }

    fn add_public_key(&self, addr: String, raw: &str) -> Result<()> {
        let mut public_keys = self
            .public_keys
            .lock()
            .map_err(|_| Error::AddPublicKeyError)?;

        let pubkey =
            RsaPublicKey::from_pkcs1_der(&hex::decode(raw).or(Err(Error::ImportPublicKeyError))?)
                .or(Err(Error::ImportPrivateKeyError))?;

        public_keys.insert(addr, pubkey);
        Ok(())
    }

    fn digest(&self, text: &[u8]) -> SecretDigest {
        Sha1::digest(text).to_vec()
    }

    fn export_public_key(&self, addr: Option<&str>) -> Result<String> {
        let public_keys = self
            .public_keys
            .lock()
            .map_err(|_| Error::ReadPublicKeyError)?;
        let pubkey = match addr {
            Some(addr) => public_keys.get(addr).ok_or(Error::PublicKeyNotfound)?,
            None => &self.default_public_key,
        };
        Ok(hex::encode(
            pubkey.to_pkcs1_der().or(Err(Error::EncodeFailed))?.as_der(),
        ))
    }
}

/// Verify a public key.
// pub fn verify_address_signed(public_key: String, message: &[u8], signature: &[u8]) -> Result<()> {
//     let pubkey = RsaPublicKey::from_pkcs1_pem(&public_key).or(Err(Error::ImportPublicKeyError))?;
//     let padding = PaddingScheme::new_pkcs1v15_sign(Some(rsa::Hash::SHA1));
//     let hashed = Sha1::digest(message).to_vec();
//     pubkey
//         .verify(padding, &hashed, signature)
//         .map_err(|e| Error::VerifyFailed(e.to_string()))
// }

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use race_core::{error::Result, secret::SecretState};

    #[test]
    fn test_sign_verify() {
        let e = Encryptor::default();
        let text = b"hello";
        let sig = e.sign_raw(text).expect("Failed to sign");
        e.verify_raw(None, text, &sig).expect("Failed to verify");
    }

    #[test]
    fn test_encrypt_decrypt() {
        let e = Encryptor::default();
        let text = b"hello";
        let encrypted = e.encrypt(None, text).expect("Failed to encrypt");
        let decrypted = e.decrypt(&encrypted[..]).expect("Failed to decrypt");
        assert_eq!(decrypted, text);
    }

    #[test]
    fn test_apply() {
        let e = Encryptor::default();
        let text = b"hello";

        let secret1 = e.gen_secret();
        let secret2 = e.gen_secret();

        let mut buffer = text.clone();
        e.apply(&secret1, &mut buffer);
        e.apply(&secret2, &mut buffer);
        e.apply(&secret1, &mut buffer);
        e.apply(&secret2, &mut buffer);
        assert_eq!(&buffer, text);
    }

    #[test]
    fn test_mask_and_unmask() -> Result<()> {
        let e = Arc::new(Encryptor::default());
        let mut state = SecretState::new(e);
        state.gen_random_secrets(1, 3);
        let original_ciphertexts = vec![vec![41; 16], vec![42; 16], vec![43; 16]];
        let encrypted = state.mask(1, original_ciphertexts.clone())?;
        let decrypted = state.unmask(1, encrypted.clone())?;
        assert_ne!(original_ciphertexts, encrypted);
        assert_eq!(decrypted, original_ciphertexts);
        Ok(())
    }

    #[test]
    fn test_lock() -> Result<()> {
        let e = Arc::new(Encryptor::default());
        let mut state = SecretState::new(e);
        state.gen_random_secrets(1, 3);
        let original_ciphertexts = vec![vec![41; 16], vec![42; 16], vec![43; 16]];
        let ciphertexts_and_tests = state.lock(1, original_ciphertexts)?;
        assert_eq!(3, ciphertexts_and_tests.len());
        Ok(())
    }
}

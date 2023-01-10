//! We used an enhanced 2-role based mental poker algorithmn among a few nodes.
//! Each node can either be a player or a transactor.
//! For each node, there are two modes for randomization:
//! 1. Shuffler: participate in the shuffling, hold the secrets
//! 2. Drawer: pick the random item by index

use std::collections::HashMap;

use arrayref::{array_refs, mut_array_refs, array_ref};
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha20;
use rsa::pkcs1::ToRsaPublicKey;
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use sha1::{Digest, Sha1};

use race_core::random::{RandomMode, RandomSpec, RandomState};
use race_core::types::{Ciphertext, SecretDigest, SecretKey};
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

    #[error("invalid ciphertexts size")]
    InvalidCiphertextsSize,

    #[error("Invalid key index")]
    InvalidKeyIndex,
}

impl From<Error> for race_core::error::Error {
    fn from(e: Error) -> Self {
        race_core::error::Error::RandomizationError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn gen_rsa() -> Result<(RsaPrivateKey, RsaPublicKey)> {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    if let Ok(privkey) = RsaPrivateKey::new(&mut rng, bits) {
        let pubkey = RsaPublicKey::from(&privkey);
        Ok((privkey, pubkey))
    } else {
        Err(Error::KeyGenFailed)
    }
}

pub fn gen_secret() -> SecretKey {
    let mut secret = [0u8; 44];
    let (key, nonce) = mut_array_refs![&mut secret, 32, 12];
    key.copy_from_slice(&rand::random::<[u8; 32]>());
    nonce.copy_from_slice(&rand::random::<[u8; 12]>());
    secret.to_vec()
}

pub fn gen_chacha20() -> ChaCha20 {
    let key: [u8; 32] = rand::random();
    let nonce: [u8; 12] = rand::random();
    ChaCha20::new(&key.into(), &nonce.into())
}

pub fn export_rsa_pubkey(pubkey: &RsaPublicKey) -> Result<String> {
    pubkey.to_pkcs1_pem().or(Err(Error::EncodeFailed))
}

/// Encrypt the message use RSA public key
pub fn encrypt(pubkey: &RsaPublicKey, text: &[u8]) -> Result<Vec<u8>> {
    let mut rng = rand::thread_rng();
    pubkey
        .encrypt(&mut rng, PaddingScheme::PKCS1v15Encrypt, text)
        .map_err(|e| Error::RsaEncryptFailed(e.to_string()))
}

/// Decrypt the message use RSA private key
pub fn decrypt(privkey: &RsaPrivateKey, text: &[u8]) -> Result<Vec<u8>> {
    privkey
        .decrypt(PaddingScheme::PKCS1v15Encrypt, text)
        .map_err(|e| Error::RsaDecryptFailed(e.to_string()))
}

pub fn sign(privkey: &RsaPrivateKey, text: &[u8]) -> Result<Vec<u8>> {
    let padding = PaddingScheme::new_pkcs1v15_sign(Some(rsa::Hash::SHA1));
    let hashed = Sha1::digest(text);
    privkey
        .sign(padding, &hashed)
        .map_err(|e| Error::SignFailed(e.to_string()))
}

pub fn verify(pubkey: &RsaPublicKey, text: &[u8], signature: &[u8]) -> Result<()> {
    let padding = PaddingScheme::new_pkcs1v15_sign(Some(rsa::Hash::SHA1));
    let hashed = Sha1::digest(text).to_vec();
    pubkey
        .verify(padding, &hashed, signature)
        .map_err(|e| Error::VerifyFailed(e.to_string()))
}

pub fn apply<S: AsRef<SecretKey>>(secret: &S, buffer: &mut [u8]) {
    let secret = array_ref![secret.as_ref(), 0, 44];
    let (key, nonce) = array_refs![secret, 32, 12];
    let mut cipher = ChaCha20::new(key.into(), nonce.into());
    cipher.apply_keystream(buffer);
}

pub fn apply_multi<S: AsRef<SecretKey>>(secrets: Vec<S>, buffer: &mut [u8]) {
    for secret in secrets.into_iter() {
        apply(secret.as_ref(), buffer);
    }
}

/// Represent a private state that contains all the secrets and
/// decryption results.
#[derive(Debug)]
pub struct SecretState {
    pub mode: RandomMode,
    /// My lock keys
    pub lock_keys: Vec<SecretKey>,
    /// My mask keys
    pub mask: SecretKey,
    /// Locks received from others
    pub received: Vec<Option<SecretKey>>,
    /// Decryption results
    pub decrypted: Vec<Option<String>>,
    /// The size of randomness
    pub size: usize,
}

impl SecretState {
    pub fn from_random_state(random_state: &RandomState, mode: RandomMode) -> Self {
        SecretState::new(random_state.size, mode)
    }

    pub fn from_random_spec(random: &dyn RandomSpec, mode: RandomMode) -> Self {
        SecretState::new(random.size(), mode)
    }

    pub fn new(size: usize, mode: RandomMode) -> Self {
        let mask = gen_secret();
        let lock_keys = std::iter::repeat_with(gen_secret).take(size).collect();
        let received = std::iter::repeat_with(|| None).take(size).collect();
        let decrypted = std::iter::repeat_with(|| None).take(size).collect();
        Self {
            mode,
            lock_keys,
            mask,
            received,
            decrypted,
            size,
        }
    }

    pub fn get_key_hex(&self, index: usize) -> Result<String> {
        if let Some(key) = self.lock_keys.get(index) {
            Ok(hex::encode_upper(key))
        } else {
            Err(Error::InvalidKeyIndex)
        }
    }

    pub fn get_key(&self, index: usize) -> Result<SecretKey> {
        if let Some(key) = self.lock_keys.get(index) {
            Ok(key.clone())
        } else {
            Err(Error::InvalidKeyIndex)
        }
    }

    /// Mask the given ciphertexts using mask secret.
    pub fn mask(&mut self, mut ciphertexts: Vec<Ciphertext>) -> Result<Vec<Ciphertext>> {
        if self.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize);
        }
        ciphertexts.iter_mut().for_each(|c| {
            apply(&self.mask, c.as_mut());
        });
        Ok(ciphertexts)
    }

    pub fn unmask(&mut self, mut ciphertexts: Vec<Ciphertext>) -> Result<Vec<Ciphertext>> {
        if self.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize);
        }
        ciphertexts
            .iter_mut()
            .for_each(|c| apply(&self.mask, c.as_mut()));
        Ok(ciphertexts)
    }

    pub fn lock(
        &mut self,
        ciphertexts: Vec<Ciphertext>,
    ) -> Result<Vec<(Ciphertext, SecretDigest)>> {
        if self.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize);
        }
        let r = ciphertexts
            .into_iter()
            .enumerate()
            .map(|(i, mut c)| {
                let lock = self.lock_keys.get_mut(i).unwrap();
                let digest = Sha1::digest(&lock);
                apply(lock, c.as_mut());
                (c, digest.to_vec())
            })
            .collect();
        Ok(r)
    }
}

/// The context for secrets holder. This context is for private
/// information, should never be shared with others.
pub struct SecretContext {
    /// My public key for others for encrypting messages and verifying my signatures
    pub public_key: RsaPublicKey,
    /// The private key for decrypting messages and signing signatures
    pub private_key: RsaPrivateKey,
    /// Others' public keys for encrypting messages and verifying signatures
    pub others_public_keys: HashMap<String, RsaPublicKey>,
    /// All runtime states for secret, each item corresponds to a randomness.
    pub secret_states: Vec<SecretState>,
}

#[cfg(test)]
mod tests {
    use race_core::random::{ShuffledList, deck_of_cards};

    use super::*;

    #[test]
    fn test_sign_verify() {
        let (privkey, pubkey) = gen_rsa().expect("Failed to generate RSA keys");
        let text = b"hello";
        let sig = sign(&privkey, text).expect("Failed to sign");
        verify(&pubkey, text, &sig).expect("Failed to verify");
    }

    #[test]
    fn test_encrypt_decrypt() {
        let (privkey, pubkey) = gen_rsa().expect("Failed to generate RSA keys");
        let text = b"hello";
        let encrypted = encrypt(&pubkey, text).expect("Failed to encrypt");
        let decrypted = decrypt(&privkey, &encrypted[..]).expect("Failed to decrypt");
        assert_eq!(decrypted, text);
    }

    #[test]
    fn test_apply() {
        let text = b"hello";

        let secret1 = gen_secret();
        let secret2 = gen_secret();

        let mut buffer = text.clone();
        apply(&secret1, &mut buffer);
        apply(&secret2, &mut buffer);
        apply(&secret1, &mut buffer);
        apply(&secret2, &mut buffer);
        assert_eq!(&buffer, text);
    }

    #[test]
    fn test_secret_state() {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let state = SecretState::from_random_spec(&rnd, RandomMode::Shuffler);
        assert_eq!(3, state.received.len());
        assert_eq!(3, state.decrypted.len());
    }

    #[test]
    fn test_mask_and_unmask() -> Result<()> {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let mut state = SecretState::from_random_spec(&rnd, RandomMode::Shuffler);
        let original_ciphertexts = vec![vec![41; 16], vec![42; 16], vec![43; 16]];
        let encrypted = state.mask(original_ciphertexts.clone())?;
        let decrypted = state.unmask(encrypted.clone())?;
        assert_ne!(original_ciphertexts, encrypted);
        assert_eq!(decrypted, original_ciphertexts);
        Ok(())
    }

    #[test]
    fn test_lock() -> Result<()> {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let mut state = SecretState::from_random_spec(&rnd, RandomMode::Shuffler);
        let original_ciphertexts = vec![vec![41; 16], vec![42; 16], vec![43; 16]];
        let ciphertexts_and_tests = state.lock(original_ciphertexts)?;
        assert_eq!(3, ciphertexts_and_tests.len());
        Ok(())
    }

    // This test case is for simulating the real case in texas holdem
    // With 3 players, first 6 cards are dealt as hole cards
    // The next 5 cards are dealt as board
    #[test]
    fn test_poker_case() {
        // Initialize a secret ciphers
        let random = deck_of_cards();
        let random_state = RandomState::new(0, &random, &["Foo".into(), "Bar".into()]);
        let secret_state = SecretState::from_random_state(&random_state, RandomMode::Shuffler);

        // Realize first 6 items

        // Assign cards for players

        // Realize next 3 cards as flop street

        // Realize next 1 cards as turn street

        // Realize next 1 cards as river street

        // Reveal hole cards
    }
}

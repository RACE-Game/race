use std::collections::HashMap;

use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use chacha20::ChaCha20;
use rand;
use rsa::{
    pkcs8::{EncodePublicKey, LineEnding},
    PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey,
};
use sha1::{Digest, Sha1};

use race_core::random::RandomSpec;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    KeyGenFailed,
    EncodeFailed,
    RsaEncryptFailed(rsa::errors::Error),
    RsaDecryptFailed(rsa::errors::Error),
    SignFailed(rsa::errors::Error),
    VerifyFailed(rsa::errors::Error),
    AesEncryptFailed,
    AesDecryptFailed,
    BranchNotExist,
    CantMask,
    CantUnmask,
    CantLock,
    InvalidCiphertextsSize,
}

pub type Ciphertext = Vec<u8>;
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

pub fn gen_chacha20() -> ChaCha20 {
    let key: [u8; 32] = rand::random();
    let nonce: [u8; 12] = rand::random();
    ChaCha20::new(&key.into(), &nonce.into())
}

pub fn export_rsa_pubkey(pubkey: &RsaPublicKey) -> Result<String> {
    if let Ok(s) = EncodePublicKey::to_public_key_pem(pubkey, LineEnding::LF) {
        Ok(s)
    } else {
        Err(Error::EncodeFailed)
    }
}

/// Encrypt the message use RSA public key
pub fn encrypt(pubkey: &RsaPublicKey, text: &[u8]) -> Result<Vec<u8>> {
    let mut rng = rand::thread_rng();
    pubkey
        .encrypt(&mut rng, PaddingScheme::PKCS1v15Encrypt, text)
        .or_else(|e| Err(Error::RsaEncryptFailed(e)))
}

/// Decrypt the message use RSA private key
pub fn decrypt(privkey: &RsaPrivateKey, text: &[u8]) -> Result<Vec<u8>> {
    privkey
        .decrypt(PaddingScheme::PKCS1v15Encrypt, text)
        .or_else(|e| Err(Error::RsaDecryptFailed(e)))
}

pub fn sign(privkey: &RsaPrivateKey, text: &[u8]) -> Result<Vec<u8>> {
    let padding = PaddingScheme::new_pkcs1v15_sign::<Sha1>();
    let hashed = Sha1::digest(text);
    privkey.sign(padding, &hashed).or_else(|e| Err(Error::SignFailed(e)))
}

pub fn verify(pubkey: &RsaPublicKey, text: &[u8], signature: &Vec<u8>) -> Result<()> {
    let padding = PaddingScheme::new_pkcs1v15_sign::<Sha1>();
    let hashed = Sha1::digest(text).to_vec();
    pubkey.verify(padding, &hashed, &signature).or_else(|e| Err(Error::VerifyFailed(e)))
}

pub fn apply(cipher: &mut ChaCha20, buffer: &mut [u8]) {
    cipher.seek(0u32);
    cipher.apply_keystream(buffer);
}

/// Represent a private state that contains all the secrets and
/// decryption results.
pub struct SecretState<'a> {
    /// My key to encrypt the private key shares
    pub my_share_pub: RsaPublicKey,
    /// Keys used to decrypt the private key shares
    pub share_priv: RsaPrivateKey,
    /// Keys used to encrypt the private key shares
    pub share_pubs: HashMap<&'a str, RsaPublicKey>,
    /// My lock keys
    pub lock_keys: Vec<ChaCha20>,
    /// My mask keys
    pub mask: ChaCha20,
    /// Locks received from others
    pub received: Vec<Option<ChaCha20>>,
    /// Decryption results
    pub decrypted: Vec<Option<String>>,
    /// The size of randomness
    pub size: usize,
}

impl<'a> SecretState<'a> {
    pub fn new(rnd: &dyn RandomSpec, addr: &'a str) -> Self {
        let size = rnd.size();
        let (priv_key, pub_key) = gen_rsa().unwrap();
        let mask = gen_chacha20();
        let lock_keys = std::iter::repeat_with(gen_chacha20).take(size).collect();
        let received = std::iter::repeat_with(|| None).take(size).collect();
        let decrypted = std::iter::repeat_with(|| None).take(size).collect();
        Self {
            my_share_pub: pub_key,
            share_priv: priv_key,
            share_pubs: HashMap::new(),
            lock_keys,
            mask,
            received,
            decrypted,
            size,
        }
    }

    pub fn mask(&mut self, mut ciphertexts: Vec<Ciphertext>) -> Result<Vec<Ciphertext>> {
        if self.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize);
        }
        ciphertexts.iter_mut().for_each(|c| {
            apply(&mut self.mask, c.as_mut());
        });
        Ok(ciphertexts)
    }

    pub fn unmask(&mut self, mut ciphertexts: Vec<Ciphertext>) -> Result<Vec<Ciphertext>> {
        if self.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize);
        }
        ciphertexts.iter_mut().for_each(|c| apply(&mut self.mask, c.as_mut()));
        Ok(ciphertexts)
    }

    pub fn lock(&mut self, tester: Ciphertext, ciphertexts: Vec<Ciphertext>) -> Result<Vec<(Ciphertext, Ciphertext)>> {
        if self.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize);
        }
        let r = ciphertexts.into_iter().enumerate().map(|(i, mut c)| {
            let lock = self.lock_keys.get_mut(i).unwrap();
            let mut t = tester.clone();
            apply(lock, c.as_mut());
            apply(lock, t.as_mut());
            (c, t.into())
        }).collect();
        Ok(r)
    }

    pub fn decrypt(&mut self) {

    }
}

#[cfg(test)]
mod tests {
    use crate::random::ShuffledList;

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

        let mut cipher1 = gen_chacha20();
        let mut cipher2 = gen_chacha20();

        let mut buffer = text.clone();
        cipher1.apply_keystream(&mut buffer);
        cipher2.apply_keystream(&mut buffer);
        cipher1.seek(0u32);
        cipher2.seek(0u32);
        cipher2.apply_keystream(&mut buffer);
        cipher1.apply_keystream(&mut buffer);
        assert_eq!(&buffer, text);
    }

    #[test]
    fn test_randomizing() {
        let rnd = ShuffledList {
            options: vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()],
        };
    }

    #[test]
    fn test_secret_state() {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let state = SecretState::new(&rnd, "alice");
        assert_eq!(3, state.received.len());
        assert_eq!(3, state.decrypted.len());
    }

    #[test]
    fn test_mask_and_unmask() -> Result<()> {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let mut state = SecretState::new(&rnd, "alice");
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
        let mut state = SecretState::new(&rnd, "alice");
        let original_ciphertexts = vec![vec![41; 16], vec![42; 16], vec![43; 16]];
        let tester = vec![13; 16];
        let ciphertexts_and_tests = state.lock(tester, original_ciphertexts)?;
        assert_eq!(3, ciphertexts_and_tests.len());
        Ok(())
    }

    // This test case is for simulating the real case in texas holdem
    // With 3 players, first 6 cards are dealt as hole cards
    // The next 5 cards are dealt as board
    #[test]
    fn test_poker_case() {
        // Initialize a secret ciphers

        // Realize first 6 items

        // Assign cards for players

        // Realize next 3 cards as flop street

        // Realize next 1 cards as turn street

        // Realize next 1 cards as river street

        // Reveal hole cards
    }
}

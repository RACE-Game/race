use std::sync::Arc;

use crate::{
    encryptor::EncryptorT,
    error::{Error, Result},
    random::{RandomMode, RandomSpec, RandomState},
    types::{Ciphertext, SecretDigest, SecretKey},
};

/// Represent a private state that contains all the secrets and
/// decryption results.
#[derive(Debug)]
pub struct SecretState {
    encryptor: Arc<dyn EncryptorT>,
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
    pub fn from_random_state(
        encryptor: Arc<dyn EncryptorT>,
        random_state: &RandomState,
        mode: RandomMode,
    ) -> Self {
        SecretState::new(encryptor, random_state.size, mode)
    }

    pub fn from_random_spec(
        encryptor: Arc<dyn EncryptorT>,
        random: &dyn RandomSpec,
        mode: RandomMode,
    ) -> Self {
        SecretState::new(encryptor, random.size(), mode)
    }

    pub fn new(encryptor: Arc<dyn EncryptorT>, size: usize, mode: RandomMode) -> Self {
        let mask = encryptor.gen_secret();
        let lock_keys = std::iter::repeat_with(|| encryptor.gen_secret())
            .take(size)
            .collect();
        let received = std::iter::repeat_with(|| None).take(size).collect();
        let decrypted = std::iter::repeat_with(|| None).take(size).collect();
        Self {
            encryptor,
            mode,
            lock_keys,
            mask,
            received,
            decrypted,
            size,
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
            self.encryptor.apply(&self.mask, c.as_mut());
        });
        Ok(ciphertexts)
    }

    pub fn unmask(&mut self, mut ciphertexts: Vec<Ciphertext>) -> Result<Vec<Ciphertext>> {
        if self.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize);
        }
        ciphertexts
            .iter_mut()
            .for_each(|c| self.encryptor.apply(&self.mask, c.as_mut()));
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
                let digest = self.encryptor.digest(lock);
                self.encryptor.apply(lock, c.as_mut());
                (c, digest)
            })
            .collect();
        Ok(r)
    }
}

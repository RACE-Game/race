use std::{collections::BTreeMap, sync::Arc};

use crate::{
    encryptor::EncryptorT,
    error::{Error, Result},
    types::{Ciphertext, DecisionId, RandomId, SecretDigest, SecretKey},
};

#[derive(Debug)]
pub struct RandomSecretGroup {
    size: usize,
    mask: SecretKey,
    locks: Vec<SecretKey>,
}

/// Represent a private state contains generated secrets.
///
/// # Random Secrets
///
/// A group of secrets will be created when a new randomness is
/// initialized.  The group contains a mask secret and a list of lock
/// secrets.  Use `mask`, `unmask` and `lock` to encrypt the
/// ciphertexts from a randomness.
///
/// The mask secret should never be shared with others.  If all mask
/// secrets are shared, then the whole randomness is possibly
/// revealed.  We only share lock secrets When reveal or assign a
/// random item.
///
/// # Decision Secrets
///
/// A decision is an immutable hidden answer from a player.  We
/// generate the secret when encrypting the answer.  By sharing the
/// secret, the answer is revealed.
///
#[derive(Debug)]
pub struct SecretState {
    encryptor: Arc<dyn EncryptorT>,
    random_secrets: BTreeMap<RandomId, RandomSecretGroup>,
    decision_secrets: BTreeMap<DecisionId, SecretKey>,
}

impl SecretState {
    pub fn new(encryptor: Arc<dyn EncryptorT>) -> Self {
        Self {
            encryptor,
            random_secrets: BTreeMap::new(),
            decision_secrets: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.random_secrets.clear();
        self.decision_secrets.clear();
    }

    pub fn gen_random_secrets(&mut self, random_id: RandomId, size: usize) {
        let g = RandomSecretGroup {
            size,
            mask: self.encryptor.gen_secret(),
            locks: std::iter::repeat_with(|| self.encryptor.gen_secret())
                .take(size)
                .collect(),
        };
        self.random_secrets.insert(random_id, g);
    }

    pub fn is_random_loaded(&self, random_id: RandomId) -> bool {
        self.random_secrets.contains_key(&random_id)
    }

    pub fn is_decision_loaded(&self, decision_id: DecisionId) -> bool {
        self.decision_secrets.contains_key(&decision_id)
    }

    pub fn get_random_lock(&self, random_id: RandomId, index: usize) -> Result<SecretKey> {
        if let Some(g) = self.random_secrets.get(&random_id) {
            if let Some(k) = g.locks.get(index) {
                Ok(k.clone())
            } else {
                Err(Error::InvalidKeyIndex)
            }
        } else {
            Err(Error::InvalidRandomId)
        }
    }

    pub fn get_decision_secret(&self, decision_id: DecisionId) -> Option<SecretKey> {
        self.decision_secrets
            .get(&decision_id)
            .map(|s| s.to_owned())
    }

    pub fn mask(
        &mut self,
        random_id: RandomId,
        mut ciphertexts: Vec<Ciphertext>,
    ) -> Result<Vec<Ciphertext>> {
        let g = self
            .random_secrets
            .get(&random_id)
            .ok_or(Error::InvalidRandomId)?;

        if g.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize(g.size as _, ciphertexts.len() as _));
        }

        ciphertexts.iter_mut().for_each(|c| {
            self.encryptor.apply(&g.mask, c);
        });

        Ok(ciphertexts)
    }

    pub fn unmask(
        &mut self,
        random_id: RandomId,
        mut ciphertexts: Vec<Ciphertext>,
    ) -> Result<Vec<Ciphertext>> {
        let g = self
            .random_secrets
            .get(&random_id)
            .ok_or(Error::InvalidRandomId)?;

        if g.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize(g.size as _, ciphertexts.len() as _));
        }

        ciphertexts.iter_mut().for_each(|c| {
            self.encryptor.apply(&g.mask, c);
        });

        Ok(ciphertexts)
    }

    pub fn lock(
        &mut self,
        random_id: RandomId,
        ciphertexts: Vec<Ciphertext>,
    ) -> Result<Vec<(Ciphertext, SecretDigest)>> {
        let g = self
            .random_secrets
            .get(&random_id)
            .ok_or(Error::InvalidRandomId)?;

        if g.size != ciphertexts.len() {
            return Err(Error::InvalidCiphertextsSize(g.size as _, ciphertexts.len() as _));
        }

        Ok(ciphertexts
            .into_iter()
            .enumerate()
            .map(|(i, mut c)| {
                let lock = g.locks.get(i).unwrap();
                let digest = self.encryptor.digest(lock);
                self.encryptor.apply(lock, c.as_mut());
                (c, digest)
            })
            .collect())
    }

    pub fn encrypt_answer(
        &mut self,
        decision_id: DecisionId,
        answer: String,
    ) -> Result<(Ciphertext, SecretDigest)> {
        let secret = self.encryptor.gen_secret();
        let mut ciphertext = answer.as_bytes().to_owned();
        self.encryptor.apply(&secret, &mut ciphertext);
        let digest = self.encryptor.digest(&secret);
        self.decision_secrets.insert(decision_id, secret);
        Ok((ciphertext, digest))
    }

    pub fn list_random_secrets(&self) -> Vec<&RandomSecretGroup> {
        self.random_secrets.values().collect()
    }

    pub fn list_decision_secerts(&self) -> Vec<&SecretKey> {
        self.decision_secrets.values().collect()
    }
}

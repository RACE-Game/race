//! # Random handling
//!
//! We use Mental Poker randomization between transactors.

pub type Ciphertext = Vec<u8>;

use thiserror::Error;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {

    #[error("invalid cipher status")]
    InvalidCipherStatus,

    #[error("invalid operator")]
    InvalidOperator,

    #[error("duplicated mask")]
    DuplicatedMask,

    #[error("duplicated lock")]
    DuplicatedLock,

    #[error("can't mask")]
    CantMask,

    #[error("invalid ciphertexts")]
    InvalidCiphertexts,

    #[error("update expired")]
    UpdateExpired,

    #[error("invalid index")]
    InvalidIndex,

    #[error("ciphertext already assigned")]
    CiphertextAlreadyAssigned,
}

impl From<Error> for crate::error::Error {
    fn from(e: Error) -> Self {
        Self::RandomizationError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum RandomMode {
    Shuffler,
    Drawer
}

/// An interface for randomness
/// Since we are using P2P generated randomness, so this structure doesn't really hold the random result.
/// `Randomness` holds the option of values, the identifiers and the generation status.
pub trait RandomSpec {
    /// Get the list of options for a random value.
    fn options(&self) -> &Vec<String>;

    fn size(&self) -> usize;
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShuffledList {
    pub options: Vec<String>,
}

impl ShuffledList {
    pub fn new<S: Into<String>>(options: Vec<S>) -> Self {
        Self {
            options: options.into_iter().map(|o| o.into()).collect(),
        }
    }
}

impl RandomSpec for ShuffledList {
    fn options(&self) -> &Vec<String> {
        &self.options
    }

    fn size(&self) -> usize {
        self.options.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum MaskStatus {
    Required,
    Applied,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct Mask {
    pub status: MaskStatus,
    pub owner: String,
}

impl Mask {
    pub fn new<S: Into<String>>(owner: S) -> Self {
        Self {
            status: MaskStatus::Required,
            owner: owner.into(),
        }
    }

    pub fn is_required(&self) -> bool {
        self.status == MaskStatus::Required
    }

    pub fn is_removed(&self) -> bool {
        self.status == MaskStatus::Removed
    }

    pub fn belongs_to<S: AsRef<str>>(&self, addr: S) -> bool {
        self.owner.eq(addr.as_ref())
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct Lock {
    pub test: Ciphertext,
    pub owner: String,
}

impl Lock {
    pub fn new<S: Into<String>>(owner: S, test: Ciphertext) -> Self {
        Self {
            test,
            owner: owner.into(),
        }
    }
}

/// The representation for a ciphertext with locks applied.
/// If all locks required are applied, then it's ready.
#[derive(Debug, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct LockedCiphertext {
    pub locks: Vec<Lock>,
    pub owner: Option<String>,
    pub ciphertext: Ciphertext,
}

impl LockedCiphertext {
    pub fn new(text: Ciphertext) -> Self {
        Self {
            locks: vec![],
            owner: None,
            ciphertext: text,
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum CipherStatus {
    #[default]
    Ready,
    Locking,
    Masking,
}

/// RandomState represents the public information for a single randomness.
#[derive(Default, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct RandomState {
    pub serial: u32,
    pub status: CipherStatus,
    pub masks: Vec<Mask>,
    pub ciphertexts: Vec<LockedCiphertext>,
}

impl RandomState {
    fn is_fully_masked(&self) -> bool {
        self.masks.iter().all(|m| !m.is_required())
    }

    fn is_fully_locked(&self) -> bool {
        self.masks.iter().all(|m| !m.is_removed())
    }

    fn get_ciphertext_mut(&mut self, index: usize) -> Option<&mut LockedCiphertext> {
        self.ciphertexts.get_mut(index)
    }

    pub fn new(rnd: &dyn RandomSpec, owners: &[String]) -> Self {
        let options = rnd.options();
        let ciphertexts = options
            .iter()
            .map(|o| {
                let ciphertext = o.as_bytes().to_owned();
                LockedCiphertext::new(ciphertext)
            })
            .collect();
        let masks = owners.into_iter().map(|o| Mask::new(o)).collect();
        Self {
            serial: 0,
            masks,
            status: CipherStatus::Masking,
            ciphertexts,
        }
    }

    pub fn mask<S: AsRef<str>>(&mut self, addr: S, mut ciphertexts: Vec<Ciphertext>) -> Result<()> {
        if self.status.ne(&CipherStatus::Masking) {
            return Err(Error::InvalidCipherStatus);
        }
        if let Some(mut mask) = self.masks.iter_mut().find(|m| m.owner.eq(addr.as_ref())) {
            if !mask.is_required() {
                return Err(Error::DuplicatedMask);
            } else {
                mask.status = MaskStatus::Applied;
                if ciphertexts.len() != self.ciphertexts.len() {
                    return Err(Error::InvalidCiphertexts);
                }
                for c in self.ciphertexts.iter_mut() {
                    c.ciphertext = ciphertexts.remove(0);
                }
            }
        } else {
            return Err(Error::InvalidOperator);
        }

        if self.is_fully_masked() {
            self.status = CipherStatus::Locking;
        }

        Ok(())
    }

    pub fn lock<S>(
        &mut self,
        addr: S,
        mut ciphertexts_and_tests: Vec<(Ciphertext, Ciphertext)>,
    ) -> Result<()>
    where
        S: Into<String> + AsRef<str> + Clone,
    {
        if self.status.ne(&CipherStatus::Locking) {
            return Err(Error::InvalidCipherStatus);
        }

        if let Some(mut mask) = self.masks.iter_mut().find(|m| m.owner.eq(addr.as_ref())) {
            if mask.status.eq(&MaskStatus::Removed) {
                return Err(Error::DuplicatedLock);
            }
            mask.status = MaskStatus::Removed;
            if ciphertexts_and_tests.len() != self.ciphertexts.len() {
                return Err(Error::InvalidCiphertexts);
            }
            for c in self.ciphertexts.iter_mut() {
                let (new_text, test) = ciphertexts_and_tests.remove(0);
                c.ciphertext = new_text;
                c.locks.push(Lock::new(addr.to_owned(), test));
            }
        } else {
            return Err(Error::InvalidOperator);
        }

        if self.is_fully_locked() {
            self.status = CipherStatus::Ready;
        }

        Ok(())
    }

    pub fn assign<S: Into<String>>(&mut self, addr: S, index: usize) -> Result<()> {
        if self.status.ne(&CipherStatus::Ready) {
            return Err(Error::InvalidCipherStatus);
        }
        if let Some(c) = self.get_ciphertext_mut(index) {
            if c.owner.is_some() {
                return Err(Error::CiphertextAlreadyAssigned);
            } else {
                c.owner = Some(addr.into());
            }
        } else {
            return Err(Error::InvalidIndex);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_new_random_spec() {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let state = RandomState::new(&rnd, &["alice".into(), "bob".into(), "charlie".into()]);
        assert_eq!(3, state.masks.len());
    }

    #[test]
    fn test_mask_serialize() {
        let mask = Mask::new("hello");
        let encoded = mask.try_to_vec().unwrap();
        let decoded = Mask::try_from_slice(&encoded).unwrap();
        assert_eq!(mask, decoded);
    }
}

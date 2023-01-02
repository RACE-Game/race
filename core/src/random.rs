//! # Random handling
//!
//! We use Mental Poker randomization between transactors.

pub type Ciphertext = Vec<u8>;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

    #[error("invalid mask provider")]
    InvalidMaskProvider,

    #[error("invalid lock provider")]
    InvalidLockProvider,
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
    Drawer,
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
#[derive(Debug, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
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

    pub fn ciphertext(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

#[derive(Default, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub enum CipherStatus {
    #[default]
    Ready,
    Locking(String), // The address to mask the ciphertexts
    Masking(String), // The address to lock the ciphertexts
}

/// RandomState represents the public information for a single randomness.
#[derive(Default, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub struct RandomState {
    pub id: usize,
    pub size: usize,
    pub status: CipherStatus,
    pub masks: Vec<Mask>,
    pub ciphertexts: Vec<LockedCiphertext>,
}

impl RandomState {
    pub fn is_fully_masked(&self) -> bool {
        self.masks.iter().all(|m| !m.is_required())
    }

    pub fn is_fully_locked(&self) -> bool {
        self.masks.iter().all(|m| m.is_removed())
    }

    fn get_ciphertext(&self, index: usize) -> Option<&LockedCiphertext> {
        self.ciphertexts.get(index)
    }

    fn get_ciphertext_mut(&mut self, index: usize) -> Option<&mut LockedCiphertext> {
        self.ciphertexts.get_mut(index)
    }

    pub fn new(id: usize, rnd: &dyn RandomSpec, owners: &[String]) -> Self {
        let options = rnd.options();
        let ciphertexts = options
            .iter()
            .map(|o| {
                let ciphertext = o.as_bytes().to_owned();
                LockedCiphertext::new(ciphertext)
            })
            .collect();
        let masks = owners.iter().map(Mask::new).collect();
        Self {
            id,
            size: rnd.size(),
            masks,
            status: CipherStatus::Masking(owners.first().unwrap().to_owned()),
            ciphertexts,
        }
    }

    pub fn mask<S: AsRef<str>>(&mut self, addr: S, mut ciphertexts: Vec<Ciphertext>) -> Result<()> {
        match self.status {
            CipherStatus::Masking(ref mask_addr) => {
                let addr = addr.as_ref();
                if mask_addr.ne(addr) {
                    return Err(Error::InvalidMaskProvider);
                }
                if let Some(mut mask) = self.masks.iter_mut().find(|m| m.owner.eq(addr)) {
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
                        if let Some(m) = self.masks.iter().find(|m| m.is_required()) {
                            self.status = CipherStatus::Masking(m.owner.clone());
                        } else {
                            self.status = CipherStatus::Locking(self.masks.first().unwrap().owner.clone());
                        }
                    }
                } else {
                    return Err(Error::InvalidOperator);
                }
                Ok(())
            }
            _ => Err(Error::InvalidCipherStatus),
        }
    }

    pub fn lock<S>(&mut self, addr: S, mut ciphertexts_and_tests: Vec<(Ciphertext, Ciphertext)>) -> Result<()>
    where
        S: Into<String> + AsRef<str> + Clone,
    {
        match self.status {
            CipherStatus::Locking(ref lock_addr) => {
                let addr = addr.as_ref();
                if addr.ne(lock_addr) {
                    return Err(Error::InvalidLockProvider);
                }

                if let Some(mut mask) = self.masks.iter_mut().find(|m| m.owner.eq(addr)) {
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
                    if let Some(m) = self.masks.iter().find(|m| !m.is_removed()) {
                        self.status = CipherStatus::Locking(m.owner.clone());
                    } else {
                        self.status = CipherStatus::Ready;
                    }
                } else {
                    return Err(Error::InvalidOperator);
                }
                Ok(())
            }
            _ => Err(Error::InvalidCipherStatus),
        }
    }

    pub fn assign<S>(&mut self, addr: S, indexes: Vec<usize>) -> Result<()>
    where
        S: ToOwned<Owned = String>,
    {
        if self.status.ne(&CipherStatus::Ready) {
            return Err(Error::InvalidCipherStatus);
        }

        if indexes
            .iter()
            .filter_map(|i| self.get_ciphertext(*i))
            .any(|c| c.owner.is_some())
        {
            return Err(Error::CiphertextAlreadyAssigned);
        }

        for i in indexes.into_iter() {
            if let Some(c) = self.get_ciphertext_mut(i) {
                c.owner = Some(addr.to_owned());
            }
        }
        Ok(())
    }
}

// helpers for convenience

/// Create a deck of cards.
/// Use A, 2-9, T, J, Q, K for kinds.
/// Use S(spade), D(diamond), C(club), H(heart) for suits.
pub fn deck_of_cards() -> ShuffledList {
    ShuffledList::new(vec![
        "ha", "h2", "h3", "h4", "h5", "h6", "h7", "h8", "h9", "ht", "hj", "hq", "hk", "sa", "s2", "s3", "s4", "s5",
        "s6", "s7", "s8", "s9", "st", "sj", "sq", "sk", "da", "d2", "d3", "d4", "d5", "d6", "d7", "d8", "d9", "dt",
        "dj", "dq", "dk", "ca", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9", "ct", "cj", "cq", "ck",
    ])
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_new_random_spec() {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let state = RandomState::new(0, &rnd, &["alice".into(), "bob".into(), "charlie".into()]);
        assert_eq!(3, state.masks.len());
    }

    #[test]
    fn test_mask_serialize() {
        let mask = Mask::new("hello");
        let encoded = mask.try_to_vec().unwrap();
        let decoded = Mask::try_from_slice(&encoded).unwrap();
        assert_eq!(mask, decoded);
    }

    #[test]
    fn test_mask() {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let mut state = RandomState::new(0, &rnd, &["alice".into(), "bob".into()]);
        assert_eq!(CipherStatus::Masking("alice".into()), state.status);
        state
            .mask("alice", vec![vec![1], vec![2], vec![3]])
            .expect("failed to mask");

        assert_eq!(CipherStatus::Masking("bob".into()), state.status);
        assert_eq!(false, state.is_fully_masked());
        state
            .mask("bob", vec![vec![1], vec![2], vec![3]])
            .expect("failed to mask");
        assert_eq!(CipherStatus::Locking("alice".into()), state.status);
        assert_eq!(true, state.is_fully_masked());
    }

    #[test]
    fn test_lock() {
        let rnd = ShuffledList::new(vec!["a", "b", "c"]);
        let mut state = RandomState::new(0, &rnd, &["alice".into(), "bob".into()]);
        state
            .mask("alice", vec![vec![1], vec![2], vec![3]])
            .expect("failed to mask");
        state
            .lock(
                "alice",
                vec![(vec![1], vec![1]), (vec![2], vec![2]), (vec![3], vec![3])],
            )
            .expect_err("should failed to lock");
        state
            .mask("bob", vec![vec![1], vec![2], vec![3]])
            .expect("failed to mask");
        assert_eq!(CipherStatus::Locking("alice".into()), state.status);
        state
            .lock(
                "alice",
                vec![(vec![1], vec![1]), (vec![2], vec![2]), (vec![3], vec![3])],
            )
            .expect("failed to lock");
        assert_eq!(CipherStatus::Locking("bob".into()), state.status);
        assert_eq!(false, state.is_fully_locked());
        state
            .lock("bob", vec![(vec![1], vec![1]), (vec![2], vec![2]), (vec![3], vec![3])])
            .expect("failed to lock");
        assert_eq!(CipherStatus::Ready, state.status);
        assert_eq!(true, state.is_fully_locked());
    }
}

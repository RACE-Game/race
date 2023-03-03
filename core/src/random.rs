//! # Random handling
//!
//! We use Mental Poker randomization between transactors.

use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::{Ciphertext, RandomId, SecretDigest, SecretIdent, SecretKey};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("invalid random status: {0:?}")]
    InvalidRandomStatus(RandomStatus),

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

    #[error("duplicated secret")]
    DuplicatedSecret,

    #[error("invalid secret")]
    InvalidSecret,

    #[error("randomness is not ready")]
    RandomnessNotReady,

    #[error("secrets are not ready")]
    SecretsNotReady,

    #[error("No enough servers")]
    NoEnoughServer,

    #[error("Invalid reveal operation")]
    InvalidRevealOperation,
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

    pub fn is_applied(&self) -> bool {
        self.status == MaskStatus::Applied
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
    pub digest: SecretDigest,
    pub owner: String,
}

impl Lock {
    pub fn new<S: Into<String>>(owner: S, digest: SecretDigest) -> Self {
        Self {
            digest,
            owner: owner.into(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub enum CipherOwner {
    #[default]
    Unclaimed,
    // Visible to the assigned player only
    Assigned(String),
    // Assigned to multiple players
    MultiAssigned(Vec<String>),
    Revealed,
}

/// The representation of a ciphertext with locks applied.
/// If all the required locks are applied, then it's ready.
#[derive(Debug, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub struct LockedCiphertext {
    pub locks: Vec<Lock>,
    pub owner: CipherOwner,
    pub ciphertext: Ciphertext,
}

impl LockedCiphertext {
    pub fn new(text: Ciphertext) -> Self {
        Self {
            locks: vec![],
            owner: CipherOwner::Unclaimed,
            ciphertext: text,
        }
    }

    pub fn ciphertext(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

#[derive(Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub struct Share {
    from_addr: String,
    // None means public revealed
    to_addr: Option<String>,
    index: usize,
    // None means missing
    secret: Option<SecretKey>,
}

#[derive(Default, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub enum RandomStatus {
    #[default]
    Ready,
    Locking,        // The address to mask the ciphertexts
    Masking,        // The address to lock the ciphertexts
    WaitingSecrets, // Waiting for the secrets to be shared
}

/// RandomState represents the public information for a single randomness.
#[derive(Default, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub struct RandomState {
    pub id: usize,
    pub size: usize,
    pub owners: Vec<String>,
    pub options: Vec<String>,
    pub status: RandomStatus,
    pub masks: Vec<Mask>,
    pub ciphertexts: Vec<LockedCiphertext>,
    pub secret_shares: Vec<Share>,
    pub revealed: HashMap<usize, String>,
}

impl RandomState {
    pub fn is_fully_masked(&self) -> bool {
        self.masks.iter().all(|m| !m.is_required())
    }

    pub fn is_fully_locked(&self) -> bool {
        self.masks.iter().all(|m| m.is_removed())
    }

    pub fn get_ciphertext(&self, index: usize) -> Option<&LockedCiphertext> {
        self.ciphertexts.get(index)
    }

    pub fn get_ciphertext_unchecked(&self, index: usize) -> &LockedCiphertext {
        &self.ciphertexts[index]
    }

    fn get_ciphertext_mut(&mut self, index: usize) -> Option<&mut LockedCiphertext> {
        self.ciphertexts.get_mut(index)
    }

    pub fn try_new(id: RandomId, options: Vec<String>, size: usize, owners: &[String]) -> Result<Self> {
        let ciphertexts = options
            .iter()
            .map(|o| {
                let ciphertext = o.as_bytes().to_owned();
                LockedCiphertext::new(ciphertext)
            })
            .collect();
        let masks = owners.iter().map(Mask::new).collect();
        if owners.is_empty() {
            return Err(Error::NoEnoughServer);
        }
        let status = RandomStatus::Masking;
        Ok(Self {
            id,
            size,
            masks,
            owners: owners.to_owned(),
            options: options.clone(),
            status,
            ciphertexts,
            revealed: HashMap::new(),
            secret_shares: Vec::new(),
        })
    }

    pub fn mask<S: AsRef<str>>(&mut self, addr: S, mut ciphertexts: Vec<Ciphertext>) -> Result<()> {
        match self.status {
            RandomStatus::Masking => {
                let addr = addr.as_ref();
                if let Some(mut mask) = self.masks.iter_mut().find(|m| m.owner.eq(addr)) {
                    if !mask.is_required() {
                        return Err(Error::DuplicatedMask);
                    } else {
                        if ciphertexts.len() != self.ciphertexts.len() {
                            return Err(Error::InvalidCiphertexts);
                        }
                        for c in self.ciphertexts.iter_mut() {
                            c.ciphertext = ciphertexts.remove(0);
                        }
                        mask.status = MaskStatus::Applied;
                        self.update_status();
                    }
                } else {
                    return Err(Error::InvalidOperator);
                }
                Ok(())
            }
            _ => Err(Error::InvalidRandomStatus(self.status.clone())),
        }
    }

    pub fn lock<S>(
        &mut self,
        addr: S,
        mut ciphertexts_and_digests: Vec<(Ciphertext, SecretDigest)>,
    ) -> Result<()>
    where
        S: Into<String> + AsRef<str> + Clone,
    {
        match self.status {
            RandomStatus::Locking => {
                let addr = addr.as_ref();

                if let Some(mut mask) = self.masks.iter_mut().find(|m| m.owner.eq(addr)) {
                    if mask.status.eq(&MaskStatus::Removed) {
                        return Err(Error::DuplicatedLock);
                    }
                    if ciphertexts_and_digests.len() != self.ciphertexts.len() {
                        return Err(Error::InvalidCiphertexts);
                    }
                    mask.status = MaskStatus::Removed;
                    for c in self.ciphertexts.iter_mut() {
                        let (new_text, digest) = ciphertexts_and_digests.remove(0);
                        c.ciphertext = new_text;
                        c.locks.push(Lock::new(addr.to_owned(), digest));
                    }
                    self.update_status();
                } else {
                    return Err(Error::InvalidOperator);
                }
                Ok(())
            }
            _ => Err(Error::InvalidRandomStatus(self.status.clone())),
        }
    }

    pub fn assign<S>(&mut self, addr: S, indexes: Vec<usize>) -> Result<()>
    where
        S: ToOwned<Owned = String>,
    {
        if !matches!(
            self.status,
            RandomStatus::Ready | RandomStatus::WaitingSecrets
        ) {
            return Err(Error::InvalidRandomStatus(self.status.clone()));
        }

        if indexes
            .iter()
            .filter_map(|i| self.get_ciphertext(*i))
            .any(|c| matches!(c.owner, CipherOwner::Assigned(_) | CipherOwner::Revealed))
        {
            return Err(Error::CiphertextAlreadyAssigned);
        }

        for i in indexes.into_iter() {
            if let Some(c) = self.get_ciphertext_mut(i) {
                c.owner = CipherOwner::Assigned(addr.to_owned());
            }
            let secrets = &mut self.secret_shares;
            for o in self.owners.iter() {
                secrets.push(Share {
                    from_addr: o.to_owned(),
                    to_addr: Some(addr.to_owned()),
                    index: i,
                    secret: None,
                });
            }
        }

        self.status = RandomStatus::WaitingSecrets;

        Ok(())
    }

    fn add_secret_share(&mut self, share: Share) {
        if self
            .secret_shares
            .iter()
            .find(|ss| {
                ss.from_addr.eq(&share.from_addr)
                    && ss.to_addr.eq(&share.to_addr)
                    && ss.index == share.index
            })
            .is_none()
        {
            self.secret_shares.push(share);
        }
    }
    pub fn reveal(&mut self, indexes: Vec<usize>) -> Result<()> {
        if !matches!(
            self.status,
            RandomStatus::Ready | RandomStatus::WaitingSecrets
        ) {
            return Err(Error::InvalidRandomStatus(self.status.clone()));
        }

        for i in indexes.into_iter() {
            if let Some(c) = self.get_ciphertext_mut(i) {
                if c.owner != CipherOwner::Revealed {
                    c.owner = CipherOwner::Revealed;
                    let owners: Vec<String> = self.owners.iter().map(|o| o.to_owned()).collect();
                    for o in owners.into_iter() {
                        self.add_secret_share(Share {
                            from_addr: o,
                            to_addr: None,
                            index: i,
                            secret: None,
                        })
                    }
                }
            }
        }

        self.status = RandomStatus::WaitingSecrets;
        Ok(())
    }

    pub fn list_required_secrets_by_from_addr(&self, from_addr: &str) -> Vec<SecretIdent> {
        self.secret_shares
            .iter()
            .filter(|ss| ss.secret.is_none() && ss.from_addr.eq(from_addr))
            .map(|ss| SecretIdent {
                from_addr: ss.from_addr.clone(),
                to_addr: ss.to_addr.clone(),
                random_id: self.id,
                index: ss.index,
            })
            .collect()
    }

    pub fn list_revealed_secrets(&self) -> Result<HashMap<usize, Vec<Ciphertext>>> {
        if self.status != RandomStatus::Ready {
            return Err(Error::SecretsNotReady);
        }
        let ret = self
            .secret_shares
            .iter()
            .filter(|ss| ss.to_addr.is_none())
            .fold(HashMap::new(), |mut acc, ss| {
                acc.entry(ss.index)
                    .and_modify(|v: &mut Vec<SecretKey>| {
                        v.push(ss.secret.as_ref().unwrap().clone())
                    })
                    .or_insert_with(|| vec![ss.secret.as_ref().unwrap().clone()]);
                acc
            });
        Ok(ret)
    }

    /// List all ciphertexts assigned to a specific address.
    /// Return a mapping from item index to ciphertext.
    pub fn list_assigned_ciphertexts(&self, addr: &str) -> HashMap<usize, Ciphertext> {
        self.ciphertexts
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                if matches!(&c.owner, CipherOwner::Assigned(a) if a.eq(addr)) {
                    Some((i, c.ciphertext.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn list_revealed_ciphertexts(&self) -> HashMap<usize, Ciphertext> {
        self.ciphertexts
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                if c.owner == CipherOwner::Revealed {
                    Some((i, c.ciphertext.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// List shared secrets by receiver address.
    /// Return a mapping from item index to list of secrets(each is in HEX format).
    /// Return [[Error::SecretsNotReady]] in case of any missing secret.
    pub fn list_shared_secrets(&self, to_addr: &str) -> Result<HashMap<usize, Vec<SecretKey>>> {
        if self.status.ne(&RandomStatus::Ready) {
            return Err(Error::SecretsNotReady);
        }

        Ok(self
            .secret_shares
            .iter()
            .filter(|ss| ss.to_addr.is_some() && ss.to_addr.as_ref().unwrap().eq(to_addr))
            .fold(HashMap::new(), |mut acc, ss| {
                acc.entry(ss.index)
                    .and_modify(|v: &mut Vec<SecretKey>| {
                        v.push(ss.secret.as_ref().unwrap().clone())
                    })
                    .or_insert_with(|| vec![ss.secret.as_ref().unwrap().clone()]);
                acc
            }))
    }

    pub fn add_revealed(&mut self, revealed: HashMap<usize, String>) -> Result<()> {
        for (index, value) in revealed.into_iter() {
            if index >= self.size {
                return Err(Error::InvalidIndex);
            }
            self.revealed.insert(index, value);
        }
        Ok(())
    }

    pub fn get_revealed(&self) -> &HashMap<usize, String> {
        &self.revealed
    }

    pub fn add_secret(
        &mut self,
        from_addr: String,
        to_addr: Option<String>,
        index: usize,
        secret: SecretKey,
    ) -> Result<()> {
        if let Some(secret_share) = self
            .secret_shares
            .iter_mut()
            .find(|ss| ss.from_addr.eq(&from_addr) && ss.to_addr.eq(&to_addr) && ss.index == index)
        {
            match secret_share.secret {
                None => {
                    if let Some(_ciphertext) = self.ciphertexts.get(secret_share.index) {
                        // TODO, check digest
                        // if let Some(lock) = ciphertext.locks.iter().find(|l| l.owner.eq(&from_addr)) {

                        // } else {
                        //     return Err(Error::InvalidSecret);
                        // }
                        secret_share.secret = Some(secret);
                    } else {
                        return Err(Error::InvalidSecret);
                    }
                }
                Some(_) => return Err(Error::DuplicatedSecret),
            }
        }
        self.update_status();
        Ok(())
    }

    /// Return addresses those haven't submitted operation
    pub fn list_operating_addrs(&self) -> Vec<String> {
        match self.status {
            RandomStatus::Ready => Vec::new(),
            RandomStatus::Locking => self
                .masks
                .iter()
                .filter(|m| m.is_applied())
                .map(|m| m.owner.to_owned())
                .collect(),
            RandomStatus::Masking => self
                .masks
                .iter()
                .filter(|m| m.is_required())
                .map(|m| m.owner.to_owned())
                .collect(),
            RandomStatus::WaitingSecrets => self
                .secret_shares
                .iter()
                .filter(|s| s.secret.is_none())
                .map(|s| s.from_addr.to_owned())
                .collect(),
        }
    }

    /// Update randomness status
    pub fn update_status(&mut self) {
        if self.masks.iter().any(Mask::is_required) {
            self.status = RandomStatus::Masking;
        } else if !self.masks.iter().all(Mask::is_removed) {
            self.status = RandomStatus::Locking;
        } else if self.secret_shares.iter().any(|s| s.secret.is_none()) {
            self.status = RandomStatus::WaitingSecrets;
        } else {
            self.status = RandomStatus::Ready;
        }
    }
}

// helpers for convenience

/// Create a deck of cards.
/// Use A, 2-9, T, J, Q, K for kinds.
/// Use S(spade), D(diamond), C(club), H(heart) for suits.
pub fn deck_of_cards() -> ShuffledList {
    ShuffledList::new(vec![
        "ha", "h2", "h3", "h4", "h5", "h6", "h7", "h8", "h9", "ht", "hj", "hq", "hk", "sa", "s2",
        "s3", "s4", "s5", "s6", "s7", "s8", "s9", "st", "sj", "sq", "sk", "da", "d2", "d3", "d4",
        "d5", "d6", "d7", "d8", "d9", "dt", "dj", "dq", "dk", "ca", "c2", "c3", "c4", "c5", "c6",
        "c7", "c8", "c9", "ct", "cj", "cq", "ck",
    ])
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_list_required_secrets() -> Result<()> {
        let mut state = RandomState::try_new(
            0,
            vec!["a".into(), "b".into(), "c".into()],
            3,
            &["alice".into(), "bob".into()],
        )?;
        state.mask("alice", vec![vec![1], vec![2], vec![3]])?;
        state.mask("bob", vec![vec![1], vec![2], vec![3]])?;
        state.lock(
            "alice",
            vec![(vec![1], vec![1]), (vec![2], vec![2]), (vec![3], vec![3])],
        )?;
        state.lock(
            "bob",
            vec![(vec![1], vec![1]), (vec![2], vec![2]), (vec![3], vec![3])],
        )?;
        state.reveal(vec![0])?;
        assert_eq!(1, state.list_required_secrets_by_from_addr("alice").len());
        assert_eq!(1, state.list_required_secrets_by_from_addr("bob").len());
        state.add_secret("alice".into(), None, 0, vec![1, 2, 3])?;
        // duplicated reveal
        state.reveal(vec![0])?;
        assert_eq!(0, state.list_required_secrets_by_from_addr("alice").len());
        assert_eq!(1, state.list_required_secrets_by_from_addr("bob").len());
        state.add_secret("bob".into(), None, 0, vec![1, 2, 3])?;
        assert_eq!(0, state.list_required_secrets_by_from_addr("alice").len());
        assert_eq!(0, state.list_required_secrets_by_from_addr("bob").len());
        assert_eq!(RandomStatus::Ready, state.status);
        Ok(())
    }

    #[test]
    fn test_new_random_spec() -> Result<()> {
        let state = RandomState::try_new(
            0,
            vec!["a".into(), "b".into(), "c".into()],
            3,
            &["alice".into(), "bob".into(), "charlie".into()],
        )?;
        assert_eq!(3, state.masks.len());
        Ok(())
    }

    #[test]
    fn test_mask_serialize() {
        let mask = Mask::new("hello");
        let encoded = mask.try_to_vec().unwrap();
        let decoded = Mask::try_from_slice(&encoded).unwrap();
        assert_eq!(mask, decoded);
    }

    #[test]
    fn test_mask() -> Result<()> {
        let mut state = RandomState::try_new(
            0,
            vec!["a".into(), "b".into(), "c".into()],
            3,
            &["alice".into(), "bob".into()],
        )?;
        assert_eq!(RandomStatus::Masking, state.status);
        state
            .mask("alice", vec![vec![1], vec![2], vec![3]])
            .expect("failed to mask");

        assert_eq!(RandomStatus::Masking, state.status);
        assert_eq!(false, state.is_fully_masked());
        state
            .mask("bob", vec![vec![1], vec![2], vec![3]])
            .expect("failed to mask");
        assert_eq!(RandomStatus::Locking, state.status);
        assert_eq!(true, state.is_fully_masked());
        Ok(())
    }

    #[test]
    fn test_add_secret_share() -> Result<()> {
        let mut state = RandomState::try_new(
            0,
            vec!["a".into(), "b".into(), "c".into()],
            3,
            &["alice".into(), "bob".into()],
        )?;
        let share1 = Share {
            from_addr: "alice".into(),
            to_addr: None,
            index: 0,
            secret: None,
        };
        let share2 = Share {
            from_addr: "alice".into(),
            to_addr: None,
            index: 0,
            secret: Some(vec![1]),
        };
        state.add_secret_share(share1);
        state.add_secret_share(share2);
        assert_eq!(state.secret_shares.len(), 1);
        Ok(())
    }

    #[test]
    fn test_lock() -> Result<()> {
        let mut state = RandomState::try_new(
            0,
            vec!["a".into(), "b".into(), "c".into()],
            3,
            &["alice".into(), "bob".into()],
        )?;
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
        assert_eq!(RandomStatus::Locking, state.status);
        state
            .lock(
                "alice",
                vec![(vec![1], vec![1]), (vec![2], vec![2]), (vec![3], vec![3])],
            )
            .expect("failed to lock");
        assert_eq!(RandomStatus::Locking, state.status);
        assert_eq!(false, state.is_fully_locked());
        state
            .lock(
                "bob",
                vec![(vec![1], vec![1]), (vec![2], vec![2]), (vec![3], vec![3])],
            )
            .expect("failed to lock");
        assert_eq!(RandomStatus::Ready, state.status);
        assert_eq!(true, state.is_fully_locked());
        Ok(())
    }
}

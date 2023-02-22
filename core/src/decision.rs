//! Decision handling
//! Player can submit an immutable decision, and hide it from seeing by others
//! Later the decision can be revealed by share the secrets.

use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;

use crate::{
    error::{Error, Result},
    types::{Ciphertext, DecisionId, SecretDigest, SecretKey},
};

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum LockedDecision {
    Asked {
        id: DecisionId,
        owner: String,
    },
    Committed {
        id: DecisionId,
        owner: String,
        digest: SecretDigest,
        ciphertext: Ciphertext,
    },
    Revealed {
        id: DecisionId,
        owner: String,
        digest: SecretDigest,
        ciphertext: Ciphertext,
        secret: SecretKey,
        value: String,
    },
    Expired {
        id: DecisionId,
        owner: String,
        digest: SecretDigest,
        ciphertext: Ciphertext,
    },
}

impl LockedDecision {
    pub fn ask(id: DecisionId, owner: String) -> Self {
        Self::Asked { id, owner }
    }

    pub fn commit(mut self, ciphertext: Ciphertext, digest: SecretDigest) -> Result<Self> {
        match self {
            LockedDecision::Asked { id, owner } => Ok(LockedDecision::Committed {
                id,
                owner,
                ciphertext,
                digest,
            }),
            _ => Err(Error::InvalidDecisionStatus),
        }
    }

    pub fn reveal(mut self, secret: SecretKey, value: String) -> Result<Self> {
        match self {
            LockedDecision::Committed {
                id,
                owner,
                digest,
                ciphertext,
            } => Ok(LockedDecision::Revealed {
                id,
                owner,
                digest,
                ciphertext,
                secret,
                value,
            }),
            _ => Err(Error::InvalidDecisionStatus),
        }
    }

    pub fn expire(mut self) -> Result<Self> {
        match self {
            LockedDecision::Committed {
                id,
                owner,
                digest,
                ciphertext,
            } => Ok(LockedDecision::Expired {
                id,
                owner,
                digest,
                ciphertext,
            }),
            _ => Err(Error::InvalidDecisionStatus),
        }
    }
}

#[derive(Default, Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct DecisionState {

}

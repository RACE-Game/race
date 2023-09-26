//! Decision handling
//!
//! Player can submit an immutable decision, and hide it from seeing by others
//! Later the decision can be revealed by share the secrets.

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    error::{Error, Result},
    types::{Ciphertext, DecisionId, SecretDigest, SecretKey},
};

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum DecisionStatus {
    Asked,
    Answered,
    Releasing,
    Released,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct Answer {
    pub digest: SecretDigest,
    pub ciphertext: Ciphertext,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct DecisionState {
    pub id: DecisionId,
    owner: String,
    status: DecisionStatus,
    answer: Option<Answer>,
    secret: Option<SecretKey>,
    value: Option<String>,
}

impl DecisionState {
    pub fn new(id: DecisionId, owner: String) -> Self {
        Self {
            id,
            owner,
            status: DecisionStatus::Asked,
            answer: None,
            secret: None,
            value: None,
        }
    }

    pub fn answer(
        &mut self,
        owner: &str,
        ciphertext: Ciphertext,
        digest: SecretDigest,
    ) -> Result<()> {
        if self.owner.ne(owner) {
            return Err(Error::InvalidDecisionOwner);
        }
        if self.status.ne(&DecisionStatus::Asked) {
            return Err(Error::InvalidDecisionStatus);
        }
        self.answer = Some(Answer { ciphertext, digest });
        self.status = DecisionStatus::Answered;
        Ok(())
    }

    pub fn release(&mut self) -> Result<()> {
        if self.status != DecisionStatus::Answered {
            Err(Error::InvalidDecisionStatus)
        } else {
            self.status = DecisionStatus::Releasing;
            Ok(())
        }
    }

    /// Add the decryption result.
    ///
    /// So that it can be read inside the game handler.
    pub fn add_released(&mut self, value: String) -> Result<()> {
        if self.status != DecisionStatus::Released {
            Err(Error::InvalidDecisionStatus)
        } else {
            self.value = Some(value);
            Ok(())
        }
    }

    /// Add the shared secret and update the status.
    ///
    /// The secret will be used to decrypt the answer.
    pub fn add_secret(&mut self, owner: &str, secret: SecretKey) -> Result<()> {
        if self.status != DecisionStatus::Releasing {
            Err(Error::InvalidDecisionStatus)
        } else if self.owner.ne(owner) {
            Err(Error::InvalidDecisionOwner)
        } else {
            self.secret = Some(secret);
            self.status = DecisionStatus::Released;
            Ok(())
        }
    }

    pub fn get_secret(&self) -> Result<&SecretKey> {
        match self.secret.as_ref() {
            Some(secret) => Ok(secret),
            None => Err(Error::InvalidDecisionStatus),
        }
    }

    pub fn is_answered(&self) -> bool {
        self.status == DecisionStatus::Answered
    }

    pub fn is_prompted(&self) -> bool {
        self.status == DecisionStatus::Asked
    }

    pub fn is_revealed(&self) -> bool {
        self.status == DecisionStatus::Released
    }

    pub fn is_revealing(&self) -> bool {
        self.status == DecisionStatus::Releasing
    }

    pub fn get_answer(&self) -> Option<&Answer> {
        self.answer.as_ref()
    }

    pub fn get_revealed(&self) -> Option<&String> {
        self.value.as_ref()
    }

    pub fn get_owner(&self) -> &str {
        self.owner.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt() {
        let st = DecisionState::new(1, "Alice".into());
        assert!(st.is_prompted());
    }

    #[test]
    fn test_answer() -> anyhow::Result<()> {
        let mut st = DecisionState::new(1, "Alice".into());
        st.answer("Alice", vec![1], vec![2])?;
        assert_eq!(
            st.answer,
            Some(Answer {
                digest: vec![2],
                ciphertext: vec![1]
            })
        );
        assert!(st.is_answered());
        Ok(())
    }

    #[test]
    fn test_reveal() -> anyhow::Result<()> {
        let mut st = DecisionState::new(1, "Alice".into());
        st.answer("Alice", vec![1], vec![2])?;
        st.release()?;
        assert!(st.is_revealing());
        assert_eq!(st.release(), Err(Error::InvalidDecisionStatus));
        Ok(())
    }

    #[test]
    fn test_add_secret() -> anyhow::Result<()> {
        let mut st = DecisionState::new(1, "Alice".into());
        st.answer("Alice", vec![1], vec![2])?;
        assert_eq!(
            st.add_secret("Alice", vec![0]),
            Err(Error::InvalidDecisionStatus)
        );
        st.release()?;
        assert_eq!(
            st.add_secret("Bob", vec![0]),
            Err(Error::InvalidDecisionOwner)
        );
        st.add_secret("Alice", vec![0])?;
        assert!(st.is_revealed());
        Ok(())
    }
}

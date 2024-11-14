use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    error::{HandleError, HandleResult},
};

/// A set of arguments for game handler initialization.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct InitAccount {
    pub max_players: u16,
    pub data: Vec<u8>,
    pub checkpoint: Option<Vec<u8>>,
}

impl InitAccount {
    pub fn data<S: BorshDeserialize>(&self) -> Result<S, HandleError> {
        S::try_from_slice(&self.data).or(Err(HandleError::MalformedGameAccountData))
    }

    /// Get deserialized checkpoint, return None if not available.
    pub fn checkpoint<T: BorshDeserialize>(&self) -> HandleResult<Option<T>> {
        self.checkpoint
            .as_ref()
            .map(|c| T::try_from_slice(c).map_err(|_| HandleError::MalformedCheckpointData))
            .transpose()
    }
}

impl Default for InitAccount {
    fn default() -> Self {
        Self {
            max_players: 0,
            data: Vec::new(),
            checkpoint: None,
        }
    }
}

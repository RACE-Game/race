use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::HandleError;

/// A set of arguments for game handler initialization.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InitAccount {
    pub max_players: u16,
    pub data: Vec<u8>,
}

impl InitAccount {
    pub fn new(max_players: u16, data: Vec<u8>) -> Self {
        Self { max_players, data }
    }

    pub fn data<S: BorshDeserialize>(&self) -> Result<S, HandleError> {
        S::try_from_slice(&self.data).or(Err(HandleError::MalformedGameAccountData))
    }
}

impl Default for InitAccount {
    fn default() -> Self {
        Self {
            max_players: 0,
            data: Vec::new(),
        }
    }
}

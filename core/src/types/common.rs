use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
pub use race_api::types::*;

pub type SettleTransferCheckpoint = (Vec<Settle>, Vec<Transfer>, Vec<u8>);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClientMode {
    Player,
    Transactor,
    Validator,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Signature {
    pub signer: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}](signer: {}, timestamp: {})",
            self.signature, self.signer, self.timestamp
        )
    }
}

use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckpointOnChain {
    pub root: Vec<u8>,
    pub size: usize,
    pub access_version: u64,
}

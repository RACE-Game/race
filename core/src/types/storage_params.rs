use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::checkpoint::CheckpointOffChain;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SaveCheckpointParams {
    pub game_addr: String,
    pub settle_version: u64,
    pub checkpoint: CheckpointOffChain,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GetCheckpointParams {
    pub game_addr: String,
    pub settle_version: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SaveResult {
    pub proof: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckpointWithProof {
    pub state: Vec<u8>,
    pub proof: Vec<u8>,
}

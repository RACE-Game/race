use borsh::{BorshDeserialize, BorshSerialize};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::checkpoint::versioned_data::VersionedData;
use crate::checkpoint::shared_data::SharedData;
use crate::checkpoint::ContextCheckpoint;

/// The checkpoint data that stores in transactor's local database.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckpointOffChain {
    pub root_data: VersionedData,
    pub shared_data: SharedData,
    pub proofs: Vec<Vec<u8>>,
}

impl CheckpointOffChain {
    pub fn to_context_checkpoint(&self) -> ContextCheckpoint {
        ContextCheckpoint {
            root_data: self.root_data.clone(),
            shared_data: self.shared_data.clone(),
        }
    }
}

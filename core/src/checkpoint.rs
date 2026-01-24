///! We have several types of checkpoints
///!
///! # CheckpointOnChain
///! The on-chain part, contains the minimal information that can be
///! used to verify the off-chain part.
///!
///! # CheckpointOffChain
///! The off-chain part, contains all the necessary information that
///! is required to recover a game context.
///!
///! The information are categorized into two parts, the shared part
///! (SharedData) and the per-game part(VersionedData).
///!
///! # ContextCheckpoint
///! A temporary checkpoint state used in game context. It is the
///! off-chain part without the proofs.
///!
///! # A note about the access_version.
///! The access version in master game's VersionedData is identical to the
///! one in the Checkpoint. It indicates the latest handled version of the player
///! information including joins & deposits.

mod offchain;
mod onchain;
mod versioned_data;
mod shared_data;
pub use offchain::CheckpointOffChain;
pub use onchain::CheckpointOnChain;
pub use versioned_data::VersionedData;
pub use shared_data::SharedData;
use crate::types::ClientMode;
use crate::error::Error;
use borsh::{BorshDeserialize, BorshSerialize};
use rs_merkle::{algorithms::Sha256, proof_serializers::ReverseHashesOrder, MerkleTree};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::node::Node;

/// Checkpoint is a combination of CheckpointOnChain and
/// CheckpointOffChain.  The on-chain part is submitted to the
/// blockchain through every settlement.  The off-chain part is saved
/// in the server's local database.  To verify the server provides a
/// valid data from its local database, a merkle tree proof is
/// required.
#[derive(Default, Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Checkpoint {
    pub root: Vec<u8>,
    pub access_version: u64,
    pub root_data: VersionedData,
    pub shared_data: SharedData,
    pub proofs: Vec<Vec<u8>>,
}

impl Checkpoint {
    pub fn new_from_parts(
        offchain_part: CheckpointOffChain,
        onchain_part: CheckpointOnChain,
    ) -> Self {
        Self {
            proofs: offchain_part.proofs,
            root_data: offchain_part.root_data,
            access_version: onchain_part.access_version,
            root: onchain_part.root,
            shared_data: offchain_part.shared_data,
        }
    }

    pub fn derive_onchain_part(&self) -> CheckpointOnChain {
        CheckpointOnChain {
            size: self.proofs.len(),
            root: self.root.clone(),
            access_version: self.access_version,
        }
    }

    pub fn derive_offchain_part(&self) -> CheckpointOffChain {
        CheckpointOffChain {
            proofs: self.proofs.clone(),
            root_data: self.root_data.clone(),
            shared_data: self.shared_data.clone(),
        }
    }
}

impl From<Checkpoint> for ContextCheckpoint {
    fn from(c: Checkpoint) -> Self {
        ContextCheckpoint {
            root_data: c.root_data,
            shared_data: c.shared_data,
        }
    }
}

/// The off-chain part of the checkpoint without proofs.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Default)]
pub struct ContextCheckpoint {
    pub root_data: VersionedData,
    pub shared_data: SharedData,
}

impl ContextCheckpoint {

    pub fn new(shared_data: SharedData, root_data: VersionedData) -> Self {
        Self { root_data, shared_data }
    }

    /// Create a new ContextCheckpoint.
    ///
    /// Panic when the `init_nodes` do not contain the transactor node.
    pub fn new_with_init_nodes(init_nodes: Vec<Node>, access_version: u64) -> Self {

        if init_nodes.iter().find(|n| n.mode == ClientMode::Transactor).is_none() {
            panic!("Can not initialize ContextCheckpoint without a transactor node");
        }

        let shared_data = SharedData {
            balances: Vec::default(),
            nodes: init_nodes,
        };

        let mut root_data = VersionedData::default();
        root_data.versions.access_version = access_version;

        Self {
            shared_data,
            root_data,
        }
    }

    pub fn shared_data(&self) -> &SharedData {
        &self.shared_data
    }

    pub fn root_data_mut(&mut self) -> &mut VersionedData {
        &mut self.root_data
    }

    pub fn root_data(&self) -> &VersionedData {
        &self.root_data
    }

    /// Build a full checkpoint that can be used in settlement.
    pub fn build_checkpoint(&self) -> Checkpoint {
        let merkle_tree = self.build_merkle_tree();

        let Some(root) = merkle_tree.root() else {
            panic!("Failed to build merkle tree, root not found");
        };

        let mut proofs: Vec<Vec<u8>> = Default::default();

        let root = root.into();

        for i in 0..merkle_tree.leaves_len() {
            let proof = merkle_tree.proof(&[i as _]);
            let proof_bs = proof.serialize::<ReverseHashesOrder>();
            proofs.insert(i, proof_bs);
        }

        Checkpoint {
            root,
            access_version: self.root_data.versions.access_version,
            root_data: self.root_data.clone(),
            shared_data: self.shared_data.clone(),
            proofs,
        }
    }

    pub fn sub_checkpoint(&self, game_id: usize) -> Result<ContextCheckpoint, Error> {
        let shared_data = self.shared_data.clone();
        let Some(versioned_data) = self.root_data.sub_data.get(&game_id) else {
            return Err(Error::MissingCheckpoint);
        };
        Ok(ContextCheckpoint::new(shared_data, versioned_data.to_owned()))
    }

    pub fn build_merkle_tree_leaves(&self) -> Vec<[u8; 32]> {
        let mut leaves: Vec<[u8; 32]> = vec![];

        self.root_data.append_to_merkle_tree_leaves(&mut leaves);

        leaves
    }

    pub fn build_merkle_tree(&self) -> MerkleTree<Sha256> {
        let leaves = self.build_merkle_tree_leaves();

        MerkleTree::from_leaves(&leaves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_spec::GameSpec;
    use crate::versions::Versions;
    use crate::entry_type::EntryType;

    #[test]
    fn test_merkle_tree() -> anyhow::Result<()> {
        let mut c = Checkpoint::default();
        let d0 = vec![1];
        let game_spec = GameSpec {
            game_addr: "test".to_string(),
            bundle_addr: "test".to_string(),
            game_id: 0,
            max_players: 6,
            entry_type: EntryType::Disabled,
        };
        c.root_data = VersionedData::new(game_spec.clone(), Versions::new(1, 1), d0);
        let d1 = vec![2];
        let d2 = vec![3];
        c.root_data.sub_data.insert(1, VersionedData::new(game_spec.clone(), Versions::new(1, 1), d1));
        c.root_data.sub_data.insert(2, VersionedData::new(game_spec.clone(), Versions::new(1, 1), d2));
        println!("checkpoint: {:?}", c);
        let c: ContextCheckpoint = c.into();
        let mt = c.build_merkle_tree();
        let root = mt.root().unwrap();
        let indices = vec![0, 1, 2];
        let proof = mt.proof(&indices);
        println!("checkpoint: {:?}", c);
        println!("merkle root: {:?}", root);
        let leaves = c.build_merkle_tree_leaves();
        println!("leaves: {:?}", leaves);
        assert!(proof.verify(root, &indices, &leaves, leaves.len()));
        Ok(())
    }
}

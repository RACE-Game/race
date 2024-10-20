use std::{collections::HashMap, fmt::Display};

use borsh::{BorshDeserialize, BorshSerialize};
use rs_merkle::{
    algorithms::Sha256, proof_serializers::ReverseHashesOrder, Hasher,
    MerkleTree,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Checkpoint represents the state snapshot of game.
/// It is used as a submission to the blockchain.
#[derive(Default, Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Checkpoint {
    pub root: Vec<u8>,
    pub access_version: u64,
    pub data: HashMap<usize, VersionedData>,
    pub proofs: HashMap<usize, Vec<u8>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckpointOnChain {
    pub root: Vec<u8>,
    pub size: usize,
    pub access_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckpointOffChain {
    pub data: HashMap<usize, VersionedData>,
    pub proofs: HashMap<usize, Vec<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VersionedData {
    pub id: usize,
    pub version: u64,
    pub data: Vec<u8>,
    pub sha: Vec<u8>,
}

impl Display for Checkpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .data
            .iter()
            .map(|(id, vd)| format!("{}#{}", id, vd.version))
            .collect::<Vec<String>>();
        write!(f, "{}", s.join(","))
    }
}

impl Checkpoint {
    pub fn new(id: usize, access_version: u64, root_version: u64, root_data: Vec<u8>) -> Self {
        let sha = Sha256::hash(&root_data);
        let mut ret = Self {
            root: Vec::new(),
            access_version,
            data: HashMap::from([(
                id,
                VersionedData {
                    id,
                    version: root_version,
                    data: root_data,
                    sha: sha.into(),
                },
            )]),
            proofs: HashMap::new(),
        };
        ret.update_root_and_proofs();
        ret
    }

    pub fn new_from_parts(offchain_part: CheckpointOffChain, onchain_part: CheckpointOnChain) -> Self {
        Self {
            proofs: offchain_part.proofs,
            data: offchain_part.data,
            access_version: onchain_part.access_version,
            root: onchain_part.root,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn update_root_and_proofs(&mut self) {
        let merkle_tree = self.to_merkle_tree();
        let root = merkle_tree
            .root()
            .expect("Expect to get root from merkle tree");
        self.root = root.into();
        let mut i = 0;
        while self.data.contains_key(&i) {
            let proof = merkle_tree.proof(&[i]);
            let proof_bs = proof.serialize::<ReverseHashesOrder>();
            self.proofs.insert(i, proof_bs);
            i += 1;
        }
    }

    pub fn get_data(&self, id: usize) -> Option<Vec<u8>> {
        self.data.get(&id).map(|d| d.data.clone())
    }

    pub fn data(&self, id: usize) -> Vec<u8> {
        self.get_data(id).unwrap_or_default()
    }

    /// Set the data of the checkpoint of game.
    pub fn set_data(&mut self, id: usize, data: Vec<u8>) {
        let sha = Sha256::hash(&data);
        if let Some(old) = self.data.get_mut(&id) {
            old.data = data;
            old.version += 1;
            old.sha = sha.into();
        } else {
            self.data.insert(id, VersionedData {
                id,
                version: 0,
                data,
                sha: sha.into(),
            });
        }
        self.update_root_and_proofs();
    }

    pub fn set_access_version(&mut self, access_version: u64) {
        self.access_version = access_version;
    }

    pub fn get_version(&self, id: usize) -> u64 {
        self.data.get(&id).map(|d| d.version).unwrap_or(0)
    }

    pub fn get_sha(&self, id: usize) -> Option<[u8; 32]> {
        self.data
            .get(&id)
            .map(|d| d.sha.clone().try_into().unwrap())
    }

    /// Get version from game with id zero.
    pub fn version(&self) -> u64 {
        self.data.get(&0).map(|d| d.version).unwrap_or(0)
    }

    pub fn to_merkle_tree(&self) -> MerkleTree<Sha256> {
        println!("Build merkle tree, current checkpoint size: {}", self.data.len());
        let mut leaves: Vec<[u8; 32]> = vec![];
        let mut i = 0;
        while let Some(vd) = self.data.get(&i) {
            leaves.push(vd.sha.clone().try_into().unwrap());
            i += 1;
        }
        MerkleTree::from_leaves(&leaves)
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn derive_onchain_part(&self) -> CheckpointOnChain {
        CheckpointOnChain {
            size: self.data.len(),
            root: self.root.clone(),
            access_version: self.access_version,
        }
    }

    pub fn derive_offchain_part(&self) -> CheckpointOffChain {
        CheckpointOffChain {
            proofs: self.proofs.clone(),
            data: self.data.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree() -> anyhow::Result<()> {
        let mut c = Checkpoint::default();
        let d0 = vec![1];
        c.maybe_init_data(0, &d0);
        let d1 = vec![2];
        c.maybe_init_data(1, &d1);
        let d2 = vec![3];
        c.maybe_init_data(2, &d2);
        println!("checkpoint: {:?}", c);
        let mt = c.to_merkle_tree();
        let root = mt.root().unwrap();
        let indices = vec![0];
        let proof = mt.proof(&indices);
        println!("checkpoint: {:?}", c);
        println!("merkle root: {:?}", root);
        let leaves = vec![c.get_sha(0).unwrap()];
        println!("leaves: {:?}", leaves);
        assert!(proof.verify(root, &indices, &leaves, c.size()));
        Ok(())
    }

    #[test]
    fn test_set_data() -> anyhow::Result<()> {
        let mut c = Checkpoint::default();
        let d = vec![1];
        c.set_data(0, d)?;
        assert_eq!(c.version(), 1);
        assert_eq!(c.data.get(&0).map(|x| x.data.clone()), Some(vec![1]));
        Ok(())
    }
}

use std::{collections::HashMap, fmt::Display};

use borsh::{BorshDeserialize, BorshSerialize};
use rs_merkle::{
    algorithms::Sha256, proof_serializers::ReverseHashesOrder, Hasher,
    MerkleTree,
};
use crate::{context::Versions, error::Error};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use race_api::{event::Event, types::GameId};
use crate::types::GameSpec;

/// Checkpoint represents the state snapshot of game.
/// It is used as a submission to the blockchain.
#[derive(Default, Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Checkpoint {
    pub root: Vec<u8>,
    pub access_version: u64,
    pub data: HashMap<GameId, VersionedData>,
    pub proofs: HashMap<GameId, Vec<u8>>,
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
    pub data: HashMap<GameId, VersionedData>,
    pub proofs: HashMap<GameId, Vec<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VersionedData {
    pub id: GameId,
    pub versions: Versions,
    pub data: Vec<u8>,
    pub sha: Vec<u8>,
    pub game_spec: GameSpec,
    pub event: Option<Event>,
}

impl Display for Checkpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .data
            .iter()
            .map(|(id, vd)| format!("{}#{:?}", id, vd.versions))
            .collect::<Vec<String>>();
        write!(f, "{}", s.join(","))
    }
}

impl Checkpoint {
    pub fn new(id: GameId, game_spec: GameSpec, versions: Versions, root_data: Vec<u8>) -> Self {
        let sha = Sha256::hash(&root_data);
        let mut ret = Self {
            root: Vec::new(),
            access_version: versions.access_version,
            data: HashMap::from([(
                id,
                VersionedData {
                    id,
                    game_spec,
                    versions,
                    data: root_data,
                    sha: sha.into(),
                    event: None,
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
        if !self.data.contains_key(&0) {
            return;
        }
        let merkle_tree = self.to_merkle_tree();
        let Some(root) = merkle_tree.root()  else {
            // Skip root update as this is not a master checkpoint
            return;
        };
        self.root = root.into();
        let mut i = 0;
        while self.data.contains_key(&i) {
            let proof = merkle_tree.proof(&[i as _]);
            let proof_bs = proof.serialize::<ReverseHashesOrder>();
            self.proofs.insert(i, proof_bs);
            i += 1;
        }
    }

    pub fn get_data(&self, id: GameId) -> Option<Vec<u8>> {
        self.data.get(&id).map(|d| d.data.clone())
    }

    pub fn get_versioned_data(&self, id: GameId) -> Option<&VersionedData> {
        self.data.get(&id)
    }

    pub fn data(&self, id: GameId) -> Vec<u8> {
        self.get_data(id).unwrap_or_default()
    }

    pub fn init_versioned_data(&mut self, versioned_data: VersionedData) -> Result<(), Error> {
        match self.data.entry(versioned_data.id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(Error::CheckpointAlreadyExists);
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(versioned_data);
                self.update_root_and_proofs();
                Ok(())
            }
        }
    }

    pub fn init_data(&mut self, id: GameId, game_spec: GameSpec, versions: Versions, data: Vec<u8>) -> Result<(), Error> {
        match self.data.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(Error::CheckpointAlreadyExists);
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                let sha = Sha256::hash(&data);

                let versioned_data = VersionedData {
                    id,
                    data,
                    sha: sha.into(),
                    game_spec,
                    versions,
                    event: None,
                };
                v.insert(versioned_data);
                self.update_root_and_proofs();
                Ok(())
            }
        }
    }

    /// Set the data of the checkpoint of game.
    pub fn set_data(&mut self, id: GameId, data: Vec<u8>) -> Result<Versions, Error> {
        let sha = Sha256::hash(&data);
        if let Some(old) = self.data.get_mut(&id) {
            old.data = data;
            old.versions.settle_version += 1;
            old.sha = sha.into();
            let versions = old.versions.clone();
            self.update_root_and_proofs();
            Ok(versions)
        } else {
            Err(Error::MissingCheckpoint)
        }
    }

    pub fn update_versioned_data(&mut self, versioned_data: VersionedData) -> Result<(), Error> {
        if let Some(old) = self.data.get_mut(&versioned_data.id) {
            *old = versioned_data;
            Ok(())
        } else {
            Err(Error::MissingCheckpoint)
        }
    }

    pub fn set_event_in_versioned_data(&mut self, id: GameId, event: Option<Event>) -> Result<(), Error> {
        if let Some(versioned_data) = self.data.get_mut(&id) {
            versioned_data.event = event;
            println!("set event in versioned data: {:?} {:?}", id, versioned_data.event);
            Ok(())
        } else {
            println!("set event in versioned data no versioned data");
            Ok(())
        }
    }

    pub fn list_versioned_data(&self) -> Vec<&VersionedData> {
        self.data.values().collect()
    }

    pub fn set_access_version(&mut self, access_version: u64) {
        self.access_version = access_version;
    }

    pub fn get_versions(&self, id: GameId) -> Option<Versions> {
        self.data.get(&id).map(|d| d.versions)
    }

    pub fn get_sha(&self, id: GameId) -> Option<[u8; 32]> {
        self.data
            .get(&id)
            .map(|d| d.sha.clone().try_into().unwrap())
    }

    pub fn to_merkle_tree(&self) -> MerkleTree<Sha256> {
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

    /// Close all subgame data, leave only the master checkpoint.
    pub fn close_sub_data(&mut self) {
        self.data.retain(|k, _| *k == 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree() -> anyhow::Result<()> {
        let mut c = Checkpoint::default();
        let d0 = vec![1];
        c.set_data(0, d0)?;
        let d1 = vec![2];
        c.set_data(0, d1)?;
        let d2 = vec![3];
        c.set_data(0, d2)?;
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
}

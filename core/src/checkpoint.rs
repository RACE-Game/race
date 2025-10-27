use std::{collections::HashMap, fmt::Display};

use race_api::event::Event;
use crate::{
    context::{DispatchEvent, Versions, Node},
    types::PlayerBalance,
    error::Error,
};
use borsh::{BorshDeserialize, BorshSerialize};
use rs_merkle::{algorithms::Sha256, proof_serializers::ReverseHashesOrder, Hasher, MerkleTree};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::GameSpec;
use race_api::effect::{EmitBridgeEvent, SubGame};

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
    pub launch_subgames: Vec<SubGame>,
    pub nodes: Vec<Node>,
    pub balances: Vec<PlayerBalance>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckpointOnChain {
    pub root: Vec<u8>,
    pub size: usize,
    pub access_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EventRecord {
    pub event: Event,
    pub timestamp: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckpointOffChain {
    pub data: HashMap<usize, VersionedData>,
    pub proofs: HashMap<usize, Vec<u8>>,
    pub launch_subgames: Vec<SubGame>,
    pub nodes: Vec<Node>,
    pub balances: Vec<PlayerBalance>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VersionedData {
    pub id: usize,
    pub versions: Versions,
    pub data: Vec<u8>,
    pub sha: Vec<u8>,
    pub game_spec: GameSpec,
    pub dispatch: Option<DispatchEvent>,
    pub bridge_events: Vec<EmitBridgeEvent>,
    pub events: Vec<EventRecord>,
}

impl VersionedData {
    fn clear_future_events(&mut self) {
        self.dispatch = None;
        self.bridge_events.clear();
    }
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
    pub fn new(id: usize, game_spec: GameSpec, versions: Versions, root_data: Vec<u8>) -> Self {
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
                    dispatch: None,
                    bridge_events: vec![],
                    events: vec![],
                },
            )]),
            proofs: HashMap::new(),
            launch_subgames: vec![],
            nodes: vec![],
            balances: vec![],
        };
        ret.update_root_and_proofs();
        ret
    }

    pub fn new_from_parts(
        offchain_part: CheckpointOffChain,
        onchain_part: CheckpointOnChain,
    ) -> Self {
        Self {
            proofs: offchain_part.proofs,
            data: offchain_part.data,
            access_version: onchain_part.access_version,
            root: onchain_part.root,
            launch_subgames: offchain_part.launch_subgames,
            nodes: offchain_part.nodes,
            balances: offchain_part.balances,
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
        let Some(root) = merkle_tree.root() else {
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

    pub fn get_data(&self, id: usize) -> Option<Vec<u8>> {
        self.data.get(&id).map(|d| d.data.clone())
    }

    pub fn get_versioned_data(&self, id: usize) -> Option<&VersionedData> {
        self.data.get(&id)
    }

    pub fn data(&self, id: usize) -> Vec<u8> {
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

    pub fn init_data(
        &mut self,
        id: usize,
        game_spec: GameSpec,
        versions: Versions,
        data: Vec<u8>,
    ) -> Result<(), Error> {
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
                    dispatch: None,
                    bridge_events: vec![],
                    events: vec![],
                };
                v.insert(versioned_data);
                self.update_root_and_proofs();
                Ok(())
            }
        }
    }

    /// Set the data of the checkpoint of game.
    pub fn set_data(&mut self, id: usize, data: Vec<u8>) -> Result<Versions, Error> {
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

    pub fn set_dispatch_in_versioned_data(
        &mut self,
        id: usize,
        dispatch: Option<DispatchEvent>,
    ) -> Result<(), Error> {
        if let Some(versioned_data) = self.data.get_mut(&id) {
            versioned_data.dispatch = dispatch;
            Ok(())
        } else {
            Err(Error::MissingCheckpoint)
        }
    }

    pub fn set_bridge_in_versioned_data(
        &mut self,
        id: usize,
        bridge_events: Vec<EmitBridgeEvent>,
    ) -> Result<(), Error> {
        if let Some(versioned_data) = self.data.get_mut(&id) {
            versioned_data.bridge_events = bridge_events;
            Ok(())
        } else {
            Err(Error::MissingCheckpoint)
        }
    }

    pub fn append_launch_subgames(&mut self, subgame: SubGame) {
        self.launch_subgames.push(subgame);
    }

    pub fn delete_launch_subgames(&mut self, id: usize) {
        self.launch_subgames.retain(|subgame| subgame.id != id);
    }

    pub fn get_launch_subgames(&self) -> Vec<SubGame> {
        self.launch_subgames.clone()
    }

    pub fn clear_future_events(&mut self) {
        self.data.values_mut().for_each(VersionedData::clear_future_events);
    }

    pub fn list_versioned_data(&self) -> Vec<&VersionedData> {
        self.data.values().collect()
    }

    pub fn set_access_version(&mut self, access_version: u64) {
        self.access_version = access_version;
    }

    pub fn get_versions(&self, id: usize) -> Option<Versions> {
        self.data.get(&id).map(|d| d.versions)
    }

    pub fn get_sha(&self, id: usize) -> Option<[u8; 32]> {
        self.data
            .get(&id)
            .map(|d| d.sha.clone().try_into().expect("Failed to get SHA"))
    }

    pub fn to_merkle_tree(&self) -> MerkleTree<Sha256> {
        let mut leaves: Vec<[u8; 32]> = vec![];
        let mut i = 0;
        while let Some(vd) = self.data.get(&i) {
            leaves.push(vd.sha.clone().try_into().expect("Failed to build merkle tree"));
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
            launch_subgames: self.launch_subgames.clone(),
            nodes: self.nodes.clone(),
            balances: self.balances.clone(),
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
        let game_spec = GameSpec {
            game_addr: "test".to_string(),
            bundle_addr: "test".to_string(),
            game_id: 0,
            max_players: 6,
        };
        c.init_data(0, game_spec.clone(), Versions::new(1, 1), d0)?;
        let d1 = vec![2];
        c.init_data(1, game_spec.clone(), Versions::new(1, 1), d1)?;
        let d2 = vec![3];
        c.init_data(2, game_spec.clone(), Versions::new(1, 1), d2)?;
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

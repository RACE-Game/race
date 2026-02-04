use borsh::{BorshDeserialize, BorshSerialize};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use rs_merkle::{algorithms::Sha256, Hasher};
use race_api::effect::EmitBridgeEvent;
use crate::dispatch_event::DispatchEvent;
use crate::game_spec::GameSpec;
use crate::versions::Versions;
use std::collections::HashMap;
use crate::error::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VersionedData {
    pub game_spec: GameSpec,
    pub versions: Versions,
    pub sub_data: HashMap<usize, VersionedData>,
    pub handler_state: Vec<u8>,
    pub dispatch: Option<DispatchEvent>,
    pub bridge_events: Vec<EmitBridgeEvent>,
}

impl VersionedData {
    pub fn new(game_spec: GameSpec, versions: Versions, handler_state: Vec<u8>) -> Self {

        Self {
            handler_state,
            game_spec,
            versions,
            ..Default::default()
        }
    }

    /// Set the state and bump the settle version.
    /// Return the updated versions.
    pub fn set_state_and_bump_version(&mut self, handler_state: Vec<u8>) {
        self.handler_state = handler_state;
        self.versions.settle_version += 1;
    }

    pub fn clear_future_events(&mut self) {
        self.dispatch = None;
        self.bridge_events.clear();
    }

    pub fn sha(&self) -> Vec<u8> {
        let bs = borsh::to_vec(self).expect("Failed to serialize VersionedData");
        Sha256::hash(&bs).into()
    }

    pub fn init_sub_data(&mut self, versioned_data: VersionedData) -> Result<(), Error> {
        match self.sub_data.entry(versioned_data.game_spec.game_id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(Error::CheckpointAlreadyExists);
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(versioned_data);
                Ok(())
            }
        }
    }

    pub fn list_sub_data(&self) -> Vec<&VersionedData> {
        self.sub_data.values().collect()
    }

    pub fn list_sub_data_mut(&mut self) -> Vec<&mut VersionedData> {
        self.sub_data.values_mut().collect()
    }

    pub fn update_sub_data(&mut self, versioned_data: VersionedData) -> Result<(), Error> {
        if let Some(old) = self.sub_data.get_mut(&versioned_data.game_spec.game_id) {
            *old = versioned_data;
            Ok(())
        } else {
            Err(Error::MissingCheckpoint)
        }
    }

    pub fn set_dispatch(
        &mut self,
        dispatch: Option<DispatchEvent>,
    ) {
        self.dispatch = dispatch;
    }

    pub fn set_bridge_events(
        &mut self,
        bridge_events: Vec<EmitBridgeEvent>,
    ) {
        self.bridge_events = bridge_events;
    }

    pub(crate) fn append_to_merkle_tree_leaves(&self, leaves: &mut Vec<[u8; 32]>) {
        let sha = self.sha();
        leaves.push(sha.try_into().expect("Failed to parse the sha vector"));
        for sub in self.sub_data.values() {
            sub.append_to_merkle_tree_leaves(leaves);
        }
    }
}

use std::{collections::HashMap, fmt::Display};

use borsh::{BorshSerialize, BorshDeserialize};
use race_api::error::{Result, Error};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Checkpoint represents the state snapshot of game.
/// It is used as a submission to the blockchain.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Checkpoint {
    pub access_version: u64,
    pub data: HashMap<usize, VersionedData>
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VersionedData {
    pub version: u64,
    pub sha: String,
    pub data: Vec<u8>,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self {
            access_version: 0,
            data: HashMap::from([
                (0, VersionedData::default())
            ])
        }
    }
}

impl Display for Checkpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.data.iter().map(|(id, vd)| format!("{}#{}", id, vd.version)).collect::<Vec<String>>();
        write!(f, "{}", s.join(","))
    }
}

impl Checkpoint {

    pub fn new(id: usize, access_version: u64, root_version: u64, root_data: &[u8]) -> Self {
        Self {
            access_version,
            data: HashMap::from([
                (id, VersionedData {
                    version: root_version,
                    sha: "".to_string(),
                    data: root_data.into(),
                })
            ])
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        self.try_to_vec().map_err(|_| Error::SerializationError)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        Self::try_from_slice(data).map_err(|_| Error::MalformedCheckpoint)
    }

    pub fn data(&self, id: usize) -> Vec<u8> {
        self.data.get(&id).map(|d| d.data.clone()).unwrap_or_else(Vec::new)
    }

    /// Set the data of the checkpoint of game.
    pub fn set_data(&mut self, id: usize, data: Vec<u8>, sha: String) -> Result<()> {
        if let Some(old) = self.data.get_mut(&id) {
            old.data = data;
            old.version += 1;
            old.sha = sha;
        }
        Ok(())
    }

    pub fn set_state_sha(&mut self, id: usize, sha: String) {
        if let Some(old) = self.data.get_mut(&id) {
            old.sha = sha;
        }
    }

    pub fn set_access_version(&mut self, access_version: u64) {
        self.access_version = access_version;
    }

    pub fn maybe_init_data(&mut self, id: usize, data: &[u8]) {
        match self.data.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => (),
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(VersionedData {
                    version: 0, // The new checkpoint data should always start from zero
                    sha: "".to_string(),
                    data: data.into()
                });
            }
        }
    }

    pub fn get_version(&self, id: usize) -> u64 {
        self.data.get(&id).map(|d| d.version).unwrap_or(0)
    }

    /// Get version from game with id zero.
    pub fn version(&self) -> u64 {
        self.data.get(&0).map(|d| d.version).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

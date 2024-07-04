use std::{collections::HashMap, fmt::Display};

use borsh::{BorshSerialize, BorshDeserialize};
use race_api::error::{Result, Error};

/// Checkpoint represents the state snapshot of game.
/// It is used as a submission to the blockchain.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Checkpoint {
    pub data: HashMap<usize, VersionedData>
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Default)]
pub struct VersionedData {
    pub version: u64,
    pub data: Vec<u8>,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self {
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
    pub fn try_new_from_slice(data: &[u8]) -> Result<Self> {
        Checkpoint::try_from_slice(data).map_err(|_| Error::DeserializeError)
    }

    pub fn new(id: usize, root_version: u64, root_data: &[u8]) -> Self {
        Self {
            data: HashMap::from([
                (id, VersionedData {
                    version: root_version,
                    data: root_data.into(),
                })
            ])
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        self.try_to_vec().map_err(|_| Error::SerializationError)
    }

    pub fn data(&self, id: usize) -> Vec<u8> {
        self.data.get(&id).map(|d| d.data.clone()).unwrap_or_else(Vec::new)
    }

    /// Set the data of the checkpoint of game.
    pub fn set_data(&mut self, id: usize, data: Vec<u8>) -> Result<()> {
        if let Some(old) = self.data.get_mut(&id) {
            old.data = data;
            old.version += 1;
        }
        Ok(())
    }

    pub fn maybe_init_data(&mut self, id: usize, data: &[u8]) {
        match self.data.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => (),
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(VersionedData {
                    version: 0, // The new checkpoint data should always start from zero
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

use std::collections::HashMap;

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
        self.data.get(&id).map(|d| d.data.clone()).unwrap_or_else(|| vec![])
    }

    pub fn set_data(&mut self, id: usize, data: Vec<u8>) -> Result<()> {
        let ver = self.version();
        let sub = self.data.get_mut(&id).ok_or(Error::InvalidSubGameId)?;
        sub.data = data;
        sub.version = ver + 1;
        self.set_version(0, ver + 1);
        Ok(())
    }

    pub fn init_sub(&mut self, id: usize, data: Vec<u8>) -> Result<()> {
        let version = self.version();
        match self.data.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(Error::InvalidSubGameId)
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(VersionedData {
                    data,
                    version
                });
            }
        }
        Ok(())
    }

    fn set_version(&mut self, id: usize, version: u64) {
        if let Some(vd) = self.data.get_mut(&id) {
            vd.version = version;
        }
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

    #[test]
    fn test_set_sub_data() -> anyhow::Result<()> {
        let mut c = Checkpoint::default();
        let d = vec![1];
        c.init_sub(1, d.clone())?;
        assert_eq!(c.version(), 0);
        c.set_data(1, d)?;
        assert_eq!(c.version(), 1);
        Ok(())
    }
}

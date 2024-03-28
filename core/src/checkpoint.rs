use std::collections::HashMap;

use borsh::{BorshSerialize, BorshDeserialize};
use race_api::error::{Result, Error};

/// Checkpoint represents the state snapshot of game.
/// It is used as a submission to the blockchain.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Default)]
pub struct Checkpoint {
    pub id: u8,
    pub data: VersionedData,
    pub subs: HashMap<u8, VersionedData>,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Default)]
pub struct VersionedData {
    pub version: u64,
    pub data: Option<Vec<u8>>,
}

impl Checkpoint {
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data.data = Some(data);
        self.data.version += 1;
    }

    pub fn set_sub_data(&mut self, sub_id: u8, data: Vec<u8>) -> Result<()> {
        let ver = self.version();
        let sub = self.subs.get_mut(&sub_id).ok_or(Error::InvalidSubGameId)?;
        sub.data = Some(data);
        sub.version = ver + 1;
        self.data.version = ver + 1;
        Ok(())
    }

    pub fn init_sub(&mut self, sub_id: u8, data: Vec<u8>) -> Result<()> {
        let version = self.version();
        match self.subs.entry(sub_id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(Error::InvalidSubGameId)
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(VersionedData {
                    data: Some(data),
                    version
                });
            }
        }
        Ok(())
    }

    pub fn version(&self) -> u64 {
        self.data.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_data() {
        let mut c = Checkpoint::default();
        let d = vec![1];
        c.set_data(d);
        assert_eq!(c.version(), 1);
        assert_eq!(c.data.data, Some(vec![1]));
    }

    #[test]
    fn test_set_sub_data() -> anyhow::Result<()> {
        let mut c = Checkpoint::default();
        let d = vec![1];
        c.init_sub(1, d.clone())?;
        assert_eq!(c.version(), 0);
        c.set_sub_data(1, d)?;
        assert_eq!(c.version(), 1);
        Ok(())
    }
}

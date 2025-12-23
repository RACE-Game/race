#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Versions {
    pub access_version: u64,
    pub settle_version: u64,
}

impl Versions {
    pub fn new(access_version: u64, settle_version: u64) -> Self {
        Self {
            access_version,
            settle_version,
        }
    }
}

impl std::fmt::Display for Versions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[s{}][a{}]", self.settle_version, self.access_version)
    }
}

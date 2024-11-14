use borsh::{BorshSerialize, BorshDeserialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The data represents the state of on-chain transactor registration.
#[derive(Debug, Default, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ServerAccount {
    // The public key of transactor owner
    pub addr: String,
    // The endpoint for transactor server
    pub endpoint: String,
}

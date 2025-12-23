use borsh::{BorshSerialize, BorshDeserialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};


#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PlayerProfile {
    pub addr: String,
    pub nick: String,
    pub pfp: Option<String>,
    pub credentials: Vec<u8>,
}

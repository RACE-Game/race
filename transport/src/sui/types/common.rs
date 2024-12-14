use serde::{Serialize, Deserialize};
use sui_sdk::types::base_types::SuiAddress;
use crate::error::{TransportError, TransportResult};
use std::str::FromStr;

// convert a str literal to SuiAddress
pub fn parse_addr(addr: &str) -> TransportResult<SuiAddress> {
    SuiAddress::from_str(addr)
        .map_err(|_| TransportError::InvalidConfig(format!("Cannot parse sui address: {}", addr)))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecipientSlotOwner {
    Unassigned { identifier: String },
    Assigned { addr: String },
}

impl TryFrom<race_core::types::RecipientSlotOwner> for RecipientSlotOwner {
    type Error = TransportError;

    fn try_from(value: race_core::types::RecipientSlotOwner) -> Result<Self, Self::Error> {
        Ok(match value {
            race_core::types::RecipientSlotOwner::Unassigned { identifier } => {
                Self::Unassigned { identifier }
            }
            race_core::types::RecipientSlotOwner::Assigned { addr } => {
                Self::Assigned { addr }
            }
        })
    }
}

impl From<RecipientSlotOwner> for race_core::types::RecipientSlotOwner {
    fn from(value: RecipientSlotOwner) -> Self {
        match value {
            RecipientSlotOwner::Unassigned { identifier } => {
                race_core::types::RecipientSlotOwner::Unassigned { identifier }
            }
            RecipientSlotOwner::Assigned { addr } => {
                race_core::types::RecipientSlotOwner::Assigned { addr }
            }
        }
    }
}

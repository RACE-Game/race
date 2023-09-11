use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

use crate::error::TransportError;

use super::parse_pubkey;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct TokenInfo {
    name: String,
    addr: String,
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum RecipientSlotOwner {
    Unassigned { identifier: String },
    Assigned { addr: Pubkey },
}

impl TryFrom<race_core::types::RecipientSlotOwner> for RecipientSlotOwner {
    type Error = TransportError;

    fn try_from(value: race_core::types::RecipientSlotOwner) -> Result<Self, Self::Error> {
        Ok(match value {
            race_core::types::RecipientSlotOwner::Unassigned { identifier } => {
                Self::Unassigned { identifier }
            }
            race_core::types::RecipientSlotOwner::Assigned { addr } => Self::Assigned {
                addr: parse_pubkey(&addr)?,
            },
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
                race_core::types::RecipientSlotOwner::Assigned {
                    addr: addr.to_string(),
                }
            }
        }
    }
}

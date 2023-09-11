use crate::solana::types::RecipientSlotOwner;
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::types::{RecipientAccount, RecipientSlotType};
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct RecipientSlotShare {
    pub owner: RecipientSlotOwner,
    pub weights: u16,
    pub claim_amount: u64,
    pub claim_amount_cap: u64,
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct RecipientSlot {
    pub id: u8,
    pub slot_type: RecipientSlotType,
    pub token_addr: Pubkey,
    pub stake_addr: Pubkey,
    pub shares: Vec<RecipientSlotShare>,
}

impl From<RecipientSlot> for race_core::types::RecipientSlot {
    fn from(value: RecipientSlot) -> Self {
        let RecipientSlot {
            id,
            slot_type,
            token_addr,
            shares,
            ..
        } = value;
        let shares = shares
            .into_iter()
            .map(|s| race_core::types::RecipientSlotShare {
                owner: s.owner.into(),
                weights: s.weights,
                claim_amount: s.claim_amount,
                claim_amount_cap: s.claim_amount_cap,
            })
            .collect();
        race_core::types::RecipientSlot {
            id,
            slot_type,
            token_addr: token_addr.to_string(),
            shares,
        }
    }
}

// State of on-chain RecipientAccount
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct RecipientState {
    pub is_initialized: bool,
    pub cap_addr: Option<Pubkey>,
    pub slots: Vec<RecipientSlot>,
}

impl RecipientState {
    pub fn into_account<S: Into<String>>(self, addr: S) -> RecipientAccount {
        let RecipientState {
            cap_addr, slots, ..
        } = self;
        RecipientAccount {
            addr: addr.into(),
            cap_addr: cap_addr.map(|a| a.to_string()),
            slots: slots
                .into_iter()
                .map(Into::<race_core::types::RecipientSlot>::into)
                .collect(),
        }
    }
}

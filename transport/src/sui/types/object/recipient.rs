use race_core::types::{RecipientAccount, RecipientSlotType, RecipientSlotOwner};
use serde::{Serialize, Deserialize};
use sui_sdk::types::{
    base_types::SuiAddress,
    transaction::Argument
};

#[derive(Serialize, Deserialize, Debug)]
pub struct RecipientBuilderWrapper {
    builder: Argument
}

impl RecipientBuilderWrapper {
    pub(crate) fn new(builder: Argument) -> Self {
        Self { builder }
    }

    pub(crate) fn update(&mut self, new_builder: Argument) {
        self.builder = new_builder;
    }

    pub(crate) fn builder(&self) -> Argument {
        self.builder.clone()
    }
}


#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Debug)]
pub struct RecipientSlotShare {
    pub owner: RecipientSlotOwner,
    pub weights: u16,
    pub claim_amount: u64,
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Debug)]
pub struct RecipientSlot {
    pub id: u8,
    pub slot_type: RecipientSlotType,
    pub token_addr: SuiAddress,
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
            })
            .collect();
        race_core::types::RecipientSlot {
            id,
            slot_type,
            token_addr: token_addr.to_string(),
            shares,
            balance: 0,                  // Set this value manually
        }
    }
}

// Object representing on-chain Recipient
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Debug)]
pub struct RecipientObject {
    pub cap_addr: Option<SuiAddress>,
    pub slots: Vec<RecipientSlot>,
}

impl RecipientObject {
    pub fn into_account<S: Into<String>>(self, addr: S) -> RecipientAccount {
        let RecipientObject {
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

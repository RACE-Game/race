use super::*;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Debug)]
pub enum RecipientSlotOwner {
    Unassigned { identifier: String },
    Assigned { addr: SuiAddress }
}

impl From<RecipientSlotOwner> for race_core::types::RecipientSlotOwner {
    fn from(value: RecipientSlotOwner) -> Self {
        match value {
            RecipientSlotOwner::Unassigned { identifier } => race_core::types::RecipientSlotOwner::Unassigned {
                identifier
            },
            RecipientSlotOwner::Assigned { addr } => race_core::types::RecipientSlotOwner::Assigned {
                addr: addr.to_string()
            },

        }
    }
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Debug)]
pub struct RecipientSlotShare {
    pub owner: RecipientSlotOwner,
    pub weights: u16,
    pub claim_amount: u64,
}

impl From<RecipientSlotShare> for race_core::types::RecipientSlotShare {
    fn from(value: RecipientSlotShare) -> Self {
        race_core::types::RecipientSlotShare {
            owner: value.owner.into(),
            weights: value.weights,
            claim_amount: value.claim_amount
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Debug)]
pub struct RecipientSlotObject {
    pub id: ObjectID,
    pub slot_id: u8,
    pub slot_type: RecipientSlotType,
    pub token_addr: String,
    pub shares: Vec<RecipientSlotShare>,
    pub balance: u64
}

impl From<RecipientSlotObject> for race_core::types::RecipientSlot {
    fn from(value: RecipientSlotObject) -> Self {
        let RecipientSlotObject {
            slot_id,
            slot_type,
            token_addr,
            shares,
            balance,
            ..
        } = value;
        let shares = shares
            .into_iter()
            .map(Into::<race_core::types::RecipientSlotShare>::into)
            .collect();
        race_core::types::RecipientSlot {
            id: slot_id,
            slot_type,
            token_addr,
            shares,
            balance,
        }
    }
}

// Struct representing on-chain Recipient object
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Debug)]
pub struct RecipientObject {
    pub id: ObjectID,
    pub cap_addr: Option<SuiAddress>,
    pub slots: Vec<RecipientSlotObject>,
}

impl From<RecipientObject> for race_core::types::RecipientAccount {
    fn from(value: RecipientObject) -> Self {
        Self {
            addr: value.id.to_hex_uncompressed(),
            cap_addr: value.cap_addr.map(|a| a.to_string()),
            slots: value.slots.into_iter().map(Into::into).collect()
        }
    }
}

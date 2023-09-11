use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct PlayerState {
    pub is_initialized: bool,
    pub nick: String, // max: 16 chars
    pub pfp: Option<Pubkey>,
}

use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct GameReg {
    pub title: String, // max: 16 chars
    pub addr: Pubkey,
    pub bundle_addr: Pubkey,
    pub reg_time: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct RegistryState {
    pub is_initialized: bool,
    pub is_private: bool,
    pub size: u16, // capacity of the registration center
    pub owner: Pubkey,
    pub games: Box<Vec<GameReg>>,
}

use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct ServerState {
    pub version: u8,
    pub addr: Pubkey,
    pub owner: Pubkey,
    pub endpoint: String, // max: 50 chars
    pub credentials: Vec<u8>,
}

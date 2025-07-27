use solana_sdk::pubkey::Pubkey;
use borsh::{BorshSerialize, BorshDeserialize};

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct PlayerJoin {
    pub addr: Pubkey,
    pub position: u16,
    pub access_version: u64,
    pub verify_key: String,
}

impl From<PlayerJoin> for race_core::types::PlayerJoin {
    fn from(value: PlayerJoin) -> Self {
        Self {
            addr: value.addr.to_string(),
            position: value.position,
            access_version: value.access_version,
            verify_key: value.verify_key,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct PlayersReg {
    pub access_version: u64,
    pub settle_version: u64,
    pub size: usize,
    pub position_flags: [u8; 128],
    pub players: Vec<PlayerJoin>,
}

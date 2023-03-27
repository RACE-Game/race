use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct GameReg {
    pub addr: Pubkey,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct RegistryState {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub games: Box<Vec<GameReg>>,
    pub padding: Box<Vec<u8>>,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct Player {
    pub addr: Pubkey,
    pub balance: u64,
    pub position: u32,
    pub access_version: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct Server {
    pub addr: Pubkey,
    pub endpoint: String,
    pub access_version: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct TokenInfo {
    pub pubkey: Pubkey,
    pub token: String,
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct GameState {
    pub is_initialized: bool,
    pub title: String,
    pub bundle_addr: Pubkey,
    pub stake_addr: Pubkey, // stake account address, to be replaced by TokenInfo?
    pub owner: Pubkey,
    pub transactor_addr: Option<Pubkey>,
    pub access_version: u64,
    pub settle_version: u64,
    pub max_players: u8,
    pub data_len: u32,
    pub data: Box<Vec<u8>>,
    pub players: Box<Vec<Player>>,
    pub servers: Box<Vec<Server>>,
    pub padding: Box<Vec<u8>>,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct PlayerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub chips: u64,
    pub nick: String, // 16 chars
    pub pfp: Option<Pubkey>,
    pub padding: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct ServerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub owner: Pubkey,
    pub endpoint: String, // max: 50 chars
    pub padding: Vec<u8>,
}

use borsh::{BorshDeserialize, BorshSerialize};
use race_api::error::Error;
use race_core::{
    checkpoint::{Checkpoint, CheckpointOnChain},
    types::{EntryType, GameAccount, VoteType},
};
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct PlayerJoin {
    pub addr: Pubkey,
    pub balance: u64,
    pub position: u16,
    pub access_version: u64,
    pub verify_key: String,
}

impl From<PlayerJoin> for race_core::types::PlayerJoin {
    fn from(value: PlayerJoin) -> Self {
        Self {
            addr: value.addr.to_string(),
            position: value.position,
            balance: value.balance,
            access_version: value.access_version,
            verify_key: value.verify_key,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct ServerJoin {
    pub addr: Pubkey,
    pub endpoint: String,
    pub access_version: u64,
    pub verify_key: String,
}

impl From<ServerJoin> for race_core::types::ServerJoin {
    fn from(value: ServerJoin) -> Self {
        Self {
            addr: value.addr.to_string(),
            endpoint: value.endpoint,
            access_version: value.access_version,
            verify_key: value.verify_key,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct Vote {
    pub voter: Pubkey,
    pub votee: Pubkey,
    pub vote_type: VoteType,
}

// State of on-chain GameAccount
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize, Debug)]
pub struct GameState {
    pub is_initialized: bool,
    // the contract version, used for upgrade
    pub version: String,
    // game name displayed on chain
    pub title: String,
    // addr to the game core logic program on Arweave
    pub bundle_addr: Pubkey,
    // addr to the account that holds all players' deposits
    pub stake_account: Pubkey,
    // game owner who created this game account
    pub owner: Pubkey,
    // mint id of the token used for game
    pub token_mint: Pubkey,
    // addr of the first server joined the game
    pub transactor_addr: Option<Pubkey>,
    // a serial number, increased by 1 after each PlayerJoin or ServerJoin
    pub access_version: u64,
    // a serial number, increased by 1 after each settlement
    pub settle_version: u64,
    // game size
    pub max_players: u16,
    // game players
    pub players: Vec<PlayerJoin>,
    // game servers (max: 10)
    pub servers: Vec<ServerJoin>,
    // length of game-specific data
    pub data_len: u32,
    // serialized data of game-specific data such as sb/bb in Texas Holdem
    pub data: Vec<u8>,
    // game votes
    pub votes: Vec<Vote>,
    // unlock time
    pub unlock_time: Option<u64>,
    // the entry type
    pub entry_type: EntryType,
    // the recipient account
    pub recipient_addr: Pubkey,
    // the checkpoint state
    pub checkpoint: Vec<u8>,
}

impl GameState {
    pub fn into_account<S: Into<String>>(self, addr: S) -> Result<GameAccount, Error> {
        let GameState {
            title,
            bundle_addr,
            owner,
            token_mint,
            transactor_addr,
            access_version,
            settle_version,
            max_players,
            players,
            servers,
            data_len,
            data,
            entry_type,
            recipient_addr,
            checkpoint,
            ..
        } = self;

        let players = players.into_iter().map(Into::into).collect();
        let servers = servers.into_iter().map(Into::into).collect();
        let checkpoint_onchain = if checkpoint.is_empty() {
            Some(CheckpointOnChain::try_from_slice(&checkpoint).map_err(|_| Error::MalformedCheckpoint)?)
        } else {
            None
        };
        let checkpoint = Checkpoint::default();

        Ok(GameAccount {
            addr: addr.into(),
            title,
            settle_version,
            bundle_addr: bundle_addr.to_string(),
            token_addr: token_mint.to_string(),
            owner_addr: owner.to_string(),
            access_version,
            players,
            servers,
            transactor_addr: transactor_addr.map(|pk| pk.to_string()),
            max_players,
            data_len,
            data,
            deposits: Vec::new(),
            votes: Vec::new(),
            unlock_time: None,
            recipient_addr: recipient_addr.to_string(),
            entry_type,
            checkpoint_on_chain: checkpoint_onchain,
            checkpoint,
        })
    }
}

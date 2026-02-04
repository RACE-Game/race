use borsh::{BorshDeserialize, BorshSerialize};
use race_core::error::Error;
use race_core::types::DepositStatus;
use race_core::{
    checkpoint::CheckpointOnChain,
    entry_type::EntryType,
    types::{EntryLock, GameAccount, VoteType},
};
use solana_sdk::pubkey::Pubkey;
use crate::solana::types::state::players::PlayerJoin;

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct PlayerDeposit {
    pub addr: Pubkey,
    pub amount: u64,
    pub access_version: u64,
    pub settle_version: u64,
    pub status: DepositStatus,
}

impl From<PlayerDeposit> for race_core::types::PlayerDeposit {
    fn from(value: PlayerDeposit) -> Self {
        Self {
            addr: value.addr.to_string(),
            amount: value.amount,
            access_version: value.access_version,
            settle_version: value.settle_version,
            status: value.status,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct ServerJoin {
    pub addr: Pubkey,
    pub endpoint: String,
    pub access_version: u64,
}

impl From<ServerJoin> for race_core::types::ServerJoin {
    fn from(value: ServerJoin) -> Self {
        Self {
            addr: value.addr.to_string(),
            endpoint: value.endpoint,
            access_version: value.access_version,
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

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct Bonus {
    pub identifier: String,
    pub stake_addr: Pubkey,
    pub token_addr: Pubkey,
    pub amount: u64,
}

impl From<Bonus> for race_core::types::Bonus {
    fn from(value: Bonus) -> Self {
        Self {
            identifier: value.identifier,
            token_addr: value.token_addr.to_string(),
            amount: value.amount,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct PlayerBalance {
    pub player_id: u64,
    pub balance: u64,
}

impl From<PlayerBalance> for race_core::types::PlayerBalance {
    fn from(value: PlayerBalance) -> Self {
        Self {
            player_id: value.player_id,
            balance: value.balance,
        }
    }
}

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, PartialEq, Eq, Clone)]
pub enum GameStatus {
    #[default]
    Initializing,
    Initialized,
    Closed,
}

// State of on-chain GameAccount
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize, Debug)]
pub struct GameState {
    pub game_status: GameStatus,
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
    // the address to players reg account
    pub players_reg_account: Pubkey,
    // player deposits
    pub deposits: Vec<PlayerDeposit>,
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
    // the lock for entry
    pub entry_lock: EntryLock,
    // a list of bonuses that can be awarded in game
    pub bonuses: Vec<Bonus>,
    // the snapshot for checkpoint balances
    pub balances: Vec<PlayerBalance>,
}

impl GameState {
    pub fn into_account<S: Into<String>>(self, addr: S, players: Vec<PlayerJoin>) -> Result<GameAccount, Error> {
        let GameState {
            title,
            bundle_addr,
            owner,
            token_mint,
            transactor_addr,
            access_version,
            settle_version,
            max_players,
            servers,
            data_len,
            data,
            entry_type,
            recipient_addr,
            checkpoint,
            entry_lock,
            deposits,
            bonuses,
            balances,
            ..
        } = self;

        let players = players.into_iter().map(Into::into).collect();
        let servers = servers.into_iter().map(Into::into).collect();
        let deposits = deposits.into_iter().map(Into::into).collect();
        let bonuses = bonuses.into_iter().map(Into::into).collect();
        let balances = balances.into_iter().map(Into::into).collect();

        let checkpoint_onchain = if !checkpoint.is_empty() {
            Some(
                CheckpointOnChain::try_from_slice(&checkpoint)
                    .map_err(|_| Error::MalformedCheckpoint)?,
            )
        } else {
            None
        };

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
            deposits,
            votes: Vec::new(),
            unlock_time: None,
            recipient_addr: recipient_addr.to_string(),
            entry_type,
            checkpoint_on_chain: checkpoint_onchain,
            entry_lock,
            bonuses,
            balances,
        })
    }
}

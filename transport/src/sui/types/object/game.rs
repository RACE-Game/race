//! Struct for on-chain game object
use super::*;

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerJoin {
    pub addr: SuiAddress,
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
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum DepositStatus {
    #[default]
    Pending,
    Rejected,
    Refunded,
    Accepted,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDeposit {
    pub addr: SuiAddress,
    pub amount: u64,
    pub access_version: u64,
    pub settle_version: u64,
    pub status: DepositStatus
}

impl From<PlayerDeposit> for race_core::types::PlayerDeposit {
    fn from(value: PlayerDeposit) -> Self {
        Self {
            addr: value.addr.to_string(),
            amount: value.amount,
            settle_version: value.settle_version,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerJoin {
    pub addr: SuiAddress,
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
#[derive(Serialize, Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Vote {
    pub voter: SuiAddress,
    pub votee: SuiAddress,
    pub vote_type: VoteType,
}

// On-chain object that represents a game
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameObject {
    // ObjectID in hex literal string format: 0x...
    pub id: ObjectID,
    // the contract version, used for upgrade
    pub version: String,
    // game name displayed on chain
    pub title: String,
    // addr to the game nft object
    pub bundle_addr: SuiAddress,
    // coin type (e.g. "0x02::sui::SUI") that holds all players' deposits in balance
    pub token_addr: String,
    // game owner who created this game account
    pub owner: SuiAddress,
    // the recipient account
    pub recipient_addr: SuiAddress,
    // addr of the first server object joined the game
    pub transactor_addr: Option<SuiAddress>,
    // a serial number, increased by 1 after each PlayerJoin or ServerJoin
    pub access_version: u64,
    // a serial number, increased by 1 after each settlement
    pub settle_version: u64,
    // game size
    pub max_players: u16,
    // game players
    pub players: Vec<PlayerJoin>,
    // player deposits
    pub deposits: Vec<PlayerDeposit>,
    // game servers (max: 10)
    pub servers: Vec<ServerJoin>,
    // game total deposits
    pub balance: u64,
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
    // the checkpoint state
    pub checkpoint: Vec<u8>,
    // the lock for entry
    pub entry_lock: EntryLock,
    // bonus ids
    pub bonuses: Vec<ObjectID>
}

impl GameObject {
    pub fn into_account(self) -> Result<GameAccount> {
        let GameObject {
            id,
            title,
            bundle_addr,
            owner,
            token_addr,
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
            entry_lock,
            deposits,
            bonuses,
            ..
        } = self;

        let players = players.into_iter().map(Into::into).collect();
        let servers = servers.into_iter().map(Into::into).collect();
        let deposits = deposits.into_iter().map(Into::into).collect();
        let checkpoint_onchain = if !checkpoint.is_empty() {
            // TODO: test this out once done settle
            Some(bcs::from_bytes(&checkpoint).map_err(|_| Error::MalformedCheckpoint)?)
        } else {
            None
        };
        let _bonuses = bonuses.into_iter().map(|b| b.to_string()).collect::<String>();
        Ok(GameAccount {
            addr: id.to_hex_uncompressed(),
            title,
            settle_version,
            bundle_addr: bundle_addr.to_string(),
            token_addr,
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
            // TODO: add bonuses
        })
    }
}

//! The data structures for on-chain accounts.

use super::{
    common::{EntryType, VoteType},
    RecipientSlot,
};
use crate::{
    checkpoint::{Checkpoint, CheckpointOnChain},
    types::{PlayerDeposit, PlayerJoin, ServerJoin},
};
use borsh::{BorshDeserialize, BorshSerialize};
use race_api::prelude::InitAccount;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Vote {
    pub voter: String,
    pub votee: String,
    pub vote_type: VoteType,
}

/// The data represents the state of on-chain transactor registration.
#[derive(Debug, Default, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ServerAccount {
    // The public key of transactor owner
    pub addr: String,
    // The endpoint for transactor server
    pub endpoint: String,
}

/// The data represents the state of on-chain recipient account.
///
/// # Cap Addr
///
/// An address which has the capacibility to manipulate the recipient
/// by adding and assigning slot.  A None value refers to an immutable
/// structure that no one can change.
///
/// # Slots
///
/// NFTs and Tokens are grouped by slots.  A slot can only store one
/// NFT or one kind of token.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RecipientAccount {
    pub addr: String,
    pub cap_addr: Option<String>,
    pub slots: Vec<RecipientSlot>,
}

/// The data represents the state of on-chain game account.
///
/// # Access Version and Settle Version
///
/// Since the blockchain and transactor are not synchronized, and the
/// RPC services usually can't provide sanitized responses, we need
/// two serial numbers to reflect when the account is updated. We also
/// rely on these versions to filter out latest events.
///
/// * After a player joined, the `access_version` will be increased by 1.
/// * After a server attached, the `access_version` will be increased by 1.
/// * After a settlement processed, the `settle_version` will be increased by 1.
/// * A deposit will use current `settle_version` + 1 to represent an unhandled operation.
///
/// # Players and Servers
///
/// Non-transactor nodes can only add themselves to the `players` list
/// or `servers` list.  Only tranactor nodes can remove a player with
/// settlement transaction.
///
/// If on-chain account requires a fixed length array to represent these lists:
/// * The max length of `players` is `max_players`.
/// * The max length of `servers` is 10.
///
/// # Deposits
///
/// The `deposits` represents a deposit from a player during the game.
/// The initial join will not produce a deposit record. The timing of
/// deposit is identified by its `settle_version`. A newly generated
/// deposit must have a higher `settle_version` which is the one in
/// game account.  Then, in the settlement, the contract will increase
/// the `settle_version` by 1, then all deposits under the version
/// will be handled as well.
///
/// Expired deposit records can be safely deleted during the
/// settlement.
///
/// # Votes
///
/// Clients and servers can vote for disconnecting.  If current
/// transactor is voted by over 50% of others, it will be downgraded
/// to a normal server.  The next server will be upgraded as
/// transactor.  The votes will be cleared at settlement.
///
/// A server or client should vote in following cases:
/// * The transactor is not responsive
/// * Event verification failed(For both timestamp or signature)
///
/// # Unlock Time
///
/// This is the timestamp used to specify when this account will be considered as unlocked.
/// Generally a game should be locked in following cases:
/// * A vote is proceed.  In this case all clients and servers are ejected.
///
/// A locked game can't be started, so settlements are disallowed.
///
/// # Data and Data Len
///
/// Data is custom-formatted data that depends on the game logic. The
/// data is used to represent the properties of a game, thus they
/// should be immutable. If a mutable state is required, it must
/// always have the same length, which is specified by `data_len`.
///
/// # Recipient address
///
/// The address to receive payment from the game.  This is used for a
/// complex payment or commission payment.
///
/// # Checkpoint
///
/// The checkpoint is the state of the game when the settlement is
/// made.  We only save the root of checkpoint merkle tree on chain.
#[derive(Debug, Default, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GameAccount {
    pub addr: String,
    pub title: String,
    pub bundle_addr: String,
    pub token_addr: String,
    pub owner_addr: String,
    pub settle_version: u64,
    pub access_version: u64,
    pub players: Vec<PlayerJoin>,
    pub deposits: Vec<PlayerDeposit>,
    pub servers: Vec<ServerJoin>,
    pub transactor_addr: Option<String>,
    pub votes: Vec<Vote>,
    pub unlock_time: Option<u64>,
    pub max_players: u16,
    pub data_len: u32,
    pub data: Vec<u8>,
    pub entry_type: EntryType,
    pub recipient_addr: String,
    pub checkpoint_on_chain: Option<CheckpointOnChain>,
}

impl GameAccount {
    pub fn derive_init_account(&self, checkpoint: &Checkpoint) -> InitAccount {
        InitAccount {
            max_players: self.max_players,
            entry_type: self.entry_type.clone(),
            players: self.players.iter().cloned().map(Into::into).collect(),
            data: self.data.clone(),
            checkpoint: checkpoint.get_data(0),
        }
    }

    pub fn derive_init_account_with_empty_checkpoint(&self) -> InitAccount {
        InitAccount {
            max_players: self.max_players,
            entry_type: self.entry_type.clone(),
            players: self.players.iter().cloned().map(Into::into).collect(),
            data: self.data.clone(),
            checkpoint: None,
        }
    }

    pub fn derive_checkpoint_init_account(&self, checkpoint: &Checkpoint) -> InitAccount {
        let players = self
            .players
            .iter()
            .filter(|p| p.access_version <= checkpoint.access_version)
            .cloned()
            .map(|p| p.into())
            .collect();

        InitAccount {
            entry_type: self.entry_type.clone(),
            max_players: self.max_players,
            players,
            data: self.data.clone(),
            checkpoint: checkpoint.get_data(0),
        }
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TokenAccount {
    pub name: String,
    pub symbol: String,
    pub icon: String,
    pub addr: String,
    pub decimals: u32,
}

#[derive(Debug, Default, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GameRegistration {
    pub title: String,
    pub addr: String,
    pub reg_time: u64,
    pub bundle_addr: String,
}

#[derive(Debug, Default, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RegistrationAccount {
    pub addr: String,
    pub is_private: bool,
    pub size: u16,
    pub owner: Option<String>, // No owner for public registration
    pub games: Vec<GameRegistration>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GameBundle {
    pub uri: String,
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PlayerProfile {
    pub addr: String,
    pub nick: String,
    pub pfp: Option<String>,
}

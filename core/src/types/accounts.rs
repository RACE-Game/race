//! The data structures for on-chain accounts.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use super::common::VoteType;

/// Represent a player call the join instruction in contract.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerJoin {
    pub addr: String,
    pub position: u16,
    pub balance: u64,
    pub access_version: u64,
}

impl PlayerJoin {
    pub fn new<S: Into<String>>(
        addr: S,
        position: u16,
        balance: u64,
        access_version: u64,
    ) -> Self {
        Self {
            addr: addr.into(),
            position,
            balance,
            access_version,
        }
    }
}

/// Represent a player call the deposit instruction in contract.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDeposit {
    pub addr: String,
    pub amount: u64,
    pub settle_version: u64,
}

impl PlayerDeposit {
    pub fn new<S: Into<String>>(addr: S, balance: u64, settle_version: u64) -> Self {
        Self {
            addr: addr.into(),
            amount: balance,
            settle_version,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerJoin {
    pub addr: String,
    pub endpoint: String,
    pub access_version: u64,
}

impl ServerJoin {
    pub fn new<S: Into<String>>(addr: S, endpoint: String, access_version: u64) -> Self {
        Self {
            addr: addr.into(),
            endpoint,
            access_version,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vote {
    pub voter: String,
    pub votee: String,
    pub vote_type: VoteType,
}

/// The data represents the state of on-chain transactor registration.
#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccount {
    // The public key of transactor owner
    pub addr: String,
    // The endpoint for transactor server
    pub endpoint: String,
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
#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
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
    pub min_deposit: u64,
    pub max_deposit: u64,
    pub data_len: u32,
    pub data: Vec<u8>,
}

#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct GameRegistration {
    pub title: String,
    pub addr: String,
    pub reg_time: u64,
    pub bundle_addr: String,
}

#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationAccount {
    pub addr: String,
    pub is_private: bool,
    pub size: u16,
    pub owner: Option<String>, // No owner for public registration
    pub games: Vec<GameRegistration>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameBundle {
    pub uri: String,
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub addr: String,
    pub nick: String,
    pub pfp: Option<String>,
}


#[cfg(test)]
mod tests {

    use crate::context::GameStatus;

    use super::*;

    #[test]
    fn test_deser() {
        let s = "{\"isPrivate\":false,\"size\":100,\"owner\":\"F6JoJgWrVZEUaRVpA2uQyRQDNdZXyhyiD8KqdfXcjXQN\",\"games\":[{\"title\":\"Raffle example\",\"addr\":\"CgZrTfRcuZ1nUbRxF6vgMnFB6ywQmj8Gai6STqwdaEae\",\"bundleAddr\":\"ES6Zpewa3XBcpBGhG7NSKgqFj7Nixzdgg21ANVs7wEUY\",\"regTime\":1680971620}]}";
        let _ra: RegistrationAccount = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_server_account() {
        let s = ServerAccount{
            addr: "an addr".to_string(),
            endpoint: "http://foo.bar".to_string(),
        };

        let res_bytes = [
            7, 0, 0, 0, 97, 110, 32, 97, 100, 100, 114, 14, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98, 97, 114
        ];
        let ser = s.try_to_vec().unwrap();
        println!("Serialized server account {:?}", ser);
        assert_eq!(ser, res_bytes);
        // let decoded = ServerAccount::try_from_slice(&res).unwrap();
        // assert_eq!(decoded, res);
    }

    #[test]
    fn test_player_profile() {
        let p = PlayerProfile {
            addr: "an addr".to_string(),
            nick: "Alice".to_string(),
            pfp: Some("Awesome PFP".to_string()),
        };
        let bytes = [7, 0, 0, 0, 97, 110, 32, 97, 100, 100, 114, 5, 0, 0, 0, 65, 108, 105, 99, 101, 1, 11, 0, 0, 0, 65, 119, 101, 115, 111, 109, 101, 32, 80, 70, 80];

        let ser = p.try_to_vec().unwrap();
        println!("Serialized player profile {:?}", ser);

        assert_eq!(ser, bytes);
    }

    #[test]
    fn test_reg() {
        let reg = RegistrationAccount {
            addr: "an addr".to_string(),
            is_private: true,
            size: 100,
            owner: Some("another addr".to_string()),
            games: vec![
                GameRegistration{
                    title: "Game A".to_string(),
                    addr: "addr 0".to_string(),
                    reg_time: 1000u64,
                    bundle_addr: "bundle 0".to_string(),
                },
                GameRegistration{
                    title: "Game B".to_string(),
                    addr: "addr 1".to_string(),
                    reg_time: 1001u64,
                    bundle_addr: "bundle 1".to_string(),
                }
            ],
        };
        let bytes = [7, 0, 0, 0, 97, 110, 32, 97, 100, 100, 114, 1, 100, 0, 1, 12, 0, 0, 0, 97, 110, 111, 116, 104, 101, 114, 32, 97, 100, 100, 114, 2, 0, 0, 0, 6, 0, 0, 0, 71, 97, 109, 101, 32, 65, 6, 0, 0, 0, 97, 100, 100, 114, 32, 48, 232, 3, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 98, 117, 110, 100, 108, 101, 32, 48, 6, 0, 0, 0, 71, 97, 109, 101, 32, 66, 6, 0, 0, 0, 97, 100, 100, 114, 32, 49, 233, 3, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 98, 117, 110, 100, 108, 101, 32, 49];

        let ser = reg.try_to_vec().unwrap();
        println!("Serialized reg {:?}", ser);

        assert_eq!(ser, bytes);
    }

    #[test]
    fn test_game_bundle() {
        let game_bunle =  GameBundle{
            uri: "http://foo.bar".to_string(),
            name: "Awesome Game".to_string(),
            data: vec![1, 2, 3, 4],
        };

        let bytes = [14, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98, 97, 114, 12, 0, 0, 0, 65, 119, 101, 115, 111, 109, 101, 32, 71, 97, 109, 101, 4, 0, 0, 0, 1, 2, 3, 4];
        let ser = game_bunle.try_to_vec().unwrap();
        println!("Serialized game bundle {:?}", ser);

        assert_eq!(ser, bytes);
    }

    #[test]
    fn test_game_account() {
        let game_account = GameAccount {
            addr: "game addr".to_string(),
            title: "awesome game title".to_string(),
            bundle_addr: "bundle addr".to_string(),
            token_addr: "token addr".to_string(),
            owner_addr: "owner addr".to_string(),
            settle_version: 10u64,
            access_version: 20u64,
            players: vec![
                PlayerJoin{
                    addr: "player 0".to_string(),
                    balance: 3u64,
                    position: 0u16,
                    access_version: 19u64,
                },
                PlayerJoin{
                    addr: "player 1".to_string(),
                    balance: 6u64,
                    position: 1u16,
                    access_version: 21u64,
                }
            ],
            deposits: vec![
                PlayerDeposit{
                    addr: "player 0".to_string(),
                    amount: 9u64,
                    settle_version: 21u64,
                },
                PlayerDeposit{
                    addr: "player 1".to_string(),
                    amount: 12u64,
                    settle_version: 21u64,
                },
            ],
            servers: vec![
                ServerJoin{
                    addr: "server 0".to_string(),
                    endpoint: "http://foo.bar".to_string(),
                    access_version: 17u64,
                },
                ServerJoin{
                    addr: "server 1".to_string(),
                    endpoint: "http://foo.bar".to_string(),
                    access_version: 17u64,
                },
            ],
            transactor_addr: Some("server 0".to_string()),
            votes: vec![
                Vote{
                    voter: "server 1".to_string(),
                    votee: "server 0".to_string(),
                    vote_type: VoteType::ServerVoteTransactorDropOff
                },
            ],
            unlock_time: None,
            max_players: 30u16,
            data_len: 10u32,
            data: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            min_deposit: 100u64,
            max_deposit: 250u64,
        };
        let bytes = [9, 0, 0, 0, 103, 97, 109, 101, 32, 97, 100, 100, 114, 18, 0, 0, 0, 97, 119, 101, 115, 111, 109, 101, 32, 103, 97, 109, 101, 32, 116, 105, 116, 108, 101, 11, 0, 0, 0, 98, 117, 110, 100, 108, 101, 32, 97, 100, 100, 114, 10, 0, 0, 0, 116, 111, 107, 101, 110, 32, 97, 100, 100, 114, 10, 0, 0, 0, 111, 119, 110, 101, 114, 32, 97, 100, 100, 114, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32, 48, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 19, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32, 49, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32, 48, 9, 0, 0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32, 49, 12, 0, 0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 48, 14, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98, 97, 114, 17, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 49, 14, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98, 97, 114, 17, 0, 0, 0, 0, 0, 0, 0, 1, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 48, 1, 0, 0, 0, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 49, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 48, 0, 0, 30, 0, 100, 0, 0, 0, 0, 0, 0, 0, 250, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let ser = game_account.try_to_vec().unwrap();
        println!("Serialized game account {:?}", ser);

        assert_eq!(ser, bytes);

    }
}

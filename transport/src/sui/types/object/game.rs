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

impl TryFromSuiMoveValue for PlayerJoin {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Struct(
                SuiMoveStruct::WithTypes { fields, ..}
                | SuiMoveStruct::WithFields(fields)
            ) => {
                Ok(Self {
                    addr: get_mv_value(fields, "addr")?,
                    position: get_mv_value(fields, "position")?,
                    access_version: get_mv_value(fields, "access_version")?,
                    verify_key: get_mv_value(fields, "verify_key")?
                })
            },
            _ => Err(Error::TransportError("expected PlayerJoin; got sth else".into()))
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDeposit {
    pub addr: SuiAddress,
    pub amount: u64,
    pub settle_version: u64,
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

impl TryFromSuiMoveValue for PlayerDeposit {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Struct(
                SuiMoveStruct::WithTypes { fields, ..}
                | SuiMoveStruct::WithFields(fields)
            ) => {
                Ok(Self {
                    addr: get_mv_value(fields, "addr")?,
                    amount: get_mv_value::<u64>(fields, "amount")?,
                    settle_version: get_mv_value::<u64>(fields, "settle_version")?,
                })
            },
            _ => Err(Error::TransportError("expected PlayerDeposit; got sth else".into()))
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

impl TryFromSuiMoveValue for ServerJoin {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Struct(
                SuiMoveStruct::WithTypes { fields, ..}
                | SuiMoveStruct::WithFields(fields)
            ) => {
                Ok(Self {
                    addr: get_mv_value(fields, "addr")?,
                    endpoint: get_mv_value::<String>(fields, "endpoint")?,
                    access_version: get_mv_value::<u64>(fields, "access_version")?,
                    verify_key: get_mv_value::<String>(fields, "verify_key")?
                })
            },
            _ => Err(Error::TransportError("expected ServerJoin; got sth else".into()))
        }
    }
}

// See the below doc comment for `EntryLock` for more detail
impl TryFromSuiMoveValue for EntryType {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Struct(sui_struct) => {
                // extract the fields BTreeMap
                let fields = match sui_struct {
                    SuiMoveStruct::WithTypes { fields, .. } => fields,
                    SuiMoveStruct::WithFields(fields) => fields,
                    _ => return Err(Error::TransportError("unexpected struct format".into()))
                };

                // determine the variant based on fields
                if fields.contains_key("min_deposit")
                    && fields.contains_key("max_deposit") {
                    Ok(Self::Cash {
                        min_deposit: get_mv_value(fields, "min_deposit")?,
                        max_deposit: get_mv_value(fields, "max_deposit")?
                    })
                } else if fields.contains_key("amount") {
                    Ok(Self::Ticket {
                        amount: get_mv_value(fields, "amount")?
                    })
                } else if fields.contains_key("collection") {
                    Ok(Self::Gating {
                        collection: get_mv_value(fields, "collection")?
                    })
                } else {
                    Ok(Self::Disabled)
                }
            },
            other => {
                println!("Expected EntryType variant but got: {:?}", other);
                Err(Error::TransportError("expected EntryType enum value".into()))
            }
        }
    }
}

/// There is a bug (#19340) of JSON API that returns a struct for what should be enum
/// or `SuiMoveValue::Varian(SuiMoveVariant)` to be more specific. For more information
/// https://github.com/MystenLabs/sui/issues/19340 (remains open on 2024-12-24)
impl TryFromSuiMoveValue for EntryLock  {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Variant(variant) => {
                match variant.variant.as_str() {
                    "Open" => Ok(Self::Open),
                    "JoinOnly" => Ok(Self::JoinOnly),
                    "DepositOnly" => Ok(Self::DepositOnly),
                    "Closed" => Ok(Self::Closed),
                    _ => Err(Error::TransportError(
                        format!("invalid EntryLock {}", variant.variant))
                    )
                }
            },
            other => {
                Err(Error::TransportError("expected EntryLock enum value".into()))
            }
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

impl TryFromSuiMoveValue for VoteType {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Variant(variant) => {
                match variant.variant.as_str() {
                    "ServerVoteTransactorDropOff" => Ok(Self::ServerVoteTransactorDropOff),
                    "ClientVoteTransactorDropOff" => Ok(Self::ClientVoteTransactorDropOff),
                    _ => Err(Error::TransportError(
                        format!("invalid VoteType {}", variant.variant))
                    )
                }
            },
            _ => Err(Error::TransportError("expected VoteType enum value".into()))
        }
    }
}

impl TryFromSuiMoveValue for Vote {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Struct(
                SuiMoveStruct::WithTypes { fields, ..}
                | SuiMoveStruct::WithFields(fields)
            ) => {
                Ok(Self {
                    voter: get_mv_value::<SuiAddress>(fields, "voter")?,
                    votee: get_mv_value::<SuiAddress>(fields, "votee")?,
                    vote_type: get_mv_value::<VoteType>(fields, "vote_type")?,
                })
            },
            _ => Err(Error::TransportError("expected Vote enum value".into()))
        }
    }
}

// On-chain object that represents a game
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Serialize, Deserialize, Debug)]
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
    pub coin_type: String,
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
}

impl TryFrom<&BTreeMap<String, SuiMoveValue>> for GameObject {
    type Error = Error;
    fn try_from(fields: &BTreeMap<String, SuiMoveValue>) -> Result<Self> {
        Ok(Self {
            id: get_mv_value(fields, "id")?,
            version: get_mv_value(fields, "version")?,
            title: get_mv_value(fields, "title")?,
            bundle_addr: get_mv_value(fields, "bundle_addr")?,
            coin_type: get_mv_value(fields, "coin_type")?,
            owner: get_mv_value(fields, "owner")?,
            recipient_addr: get_mv_value(fields, "recipient_addr")?,
            transactor_addr: get_mv_value(fields, "transactor_addr")?,
            access_version: get_mv_value(fields, "access_version")?,
            settle_version: get_mv_value(fields, "settle_version")?,
            max_players: get_mv_value(fields, "max_players")?,
            players: get_mv_value(fields, "players")?,
            deposits: get_mv_value(fields, "deposits")?,
            servers: get_mv_value(fields, "servers")?,
            data_len: get_mv_value(fields, "data_len")?,
            data: get_mv_value(fields, "data")?,
            votes: get_mv_value(fields, "votes")?,
            unlock_time: get_mv_value(fields, "unlock_time")?,
            entry_type: get_mv_value::<EntryType>(fields, "entry_type")?,
            checkpoint: get_mv_value(fields, "checkpoint")?,
            entry_lock: get_mv_value::<EntryLock>(fields, "entry_lock")?,
        })
    }
}

use race_core::types::EntryType as ET;
use race_core::types::EntryLock as EL;
impl GameObject {
    pub fn into_account<S: Into<String>>(self, addr: S) -> Result<GameAccount> {
        let GameObject {
            id,
            title,
            bundle_addr,
            owner,
            coin_type,
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
            ..
        } = self;

        let players = players.into_iter().map(Into::into).collect();
        let servers = servers.into_iter().map(Into::into).collect();
        let deposits = deposits.into_iter().map(Into::into).collect();
        let checkpoint_onchain = if !checkpoint.is_empty() {
            Some(bcs::from_bytes(&checkpoint).map_err(|_| Error::MalformedCheckpoint)?)
        } else {
            None
        };

        Ok(GameAccount {
            addr: id.to_canonical_string(true),
            title,
            settle_version,
            bundle_addr: bundle_addr.to_string(),
            token_addr: coin_type,
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
            entry_type: ET::Disabled,
            checkpoint_on_chain: checkpoint_onchain,
            entry_lock: EL::Open
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sui::utils::new_structtag;
    use sui_sdk::types::base_types::SuiAddress;
    use sui_json_rpc_types::{SuiMoveVariant, SuiMoveValue};

    #[test]
    fn test_complex_types_deserialization() -> Result<()> {
        // Test data setup
        let addr1 = SuiAddress::random_for_testing_only();
        let addr2 = SuiAddress::random_for_testing_only();

        // PlayerJoin vector test
        let player_joins = SuiMoveValue::Vector(vec![
            SuiMoveValue::Struct(SuiMoveStruct::WithTypes {
                type_: new_structtag("0x1::game::PlayerJoin")?,
                fields: BTreeMap::from([
                    ("addr".into(), SuiMoveValue::Address(addr1)),
                    ("position".into(), SuiMoveValue::Number(1)),
                    ("access_version".into(), SuiMoveValue::String("100".into())),
                    ("verify_key".into(), SuiMoveValue::String("key1".into()))
                ])
            })
        ]);

        // PlayerDeposit vector test
        let player_deposits = SuiMoveValue::Vector(vec![
            SuiMoveValue::Struct(SuiMoveStruct::WithTypes {
                type_: new_structtag("0x1::game::PlayerDeposit")?,
                fields: BTreeMap::from([
                    ("addr".into(), SuiMoveValue::Address(addr1)),
                    ("amount".into(), SuiMoveValue::String("1000".into())),
                    ("settle_version".into(), SuiMoveValue::String("1".into()))
                ])
            })
        ]);

        // ServerJoin vector test
        let server_joins = SuiMoveValue::Vector(vec![
            SuiMoveValue::Struct(SuiMoveStruct::WithTypes {
                type_: new_structtag("0x1::game::ServerJoin")?,
                fields: BTreeMap::from([
                    ("addr".into(), SuiMoveValue::Address(addr1)),
                    ("endpoint".into(), SuiMoveValue::String("http://localhost:8080".into())),
                    ("access_version".into(), SuiMoveValue::String("1".into())),
                    ("verify_key".into(), SuiMoveValue::String("server_key1".into()))
                ])
            })
        ]);

        // Vote vector test
        let votes = SuiMoveValue::Vector(vec![
            SuiMoveValue::Struct(SuiMoveStruct::WithTypes {
                type_: new_structtag("0x1::game::Vote")?,
                fields: BTreeMap::from([
                    ("voter".into(), SuiMoveValue::Address(addr1)),
                    ("votee".into(), SuiMoveValue::Address(addr2)),
                    ("vote_type".into(), SuiMoveValue::Variant(SuiMoveVariant {
                        type_: new_structtag("0x1::game::VoteType")?,
                        variant: "ServerVoteTransactorDropOff".into(),
                        fields: BTreeMap::<String, SuiMoveValue>::new()
                    }))
                ])
            })
        ]);

        // Option tests
        let some_address = SuiMoveValue::Option(Box::new(Some(SuiMoveValue::Address(addr1))));
        let none_address: SuiMoveValue = SuiMoveValue::Option(Box::new(None));
        let some_u64 = SuiMoveValue::Option(Box::new(Some(SuiMoveValue::String("100".into()))));
        let none_u64: SuiMoveValue = SuiMoveValue::Option(Box::new(None));

        // EntryType tests
        let entry_cash = SuiMoveValue::Variant(SuiMoveVariant {
            type_: new_structtag("0x1::game::EntryType")?,
            variant: "Cash".into(),
            fields: BTreeMap::from([
                ("min_deposit".into(), SuiMoveValue::String("100".into())),
                ("max_deposit".into(), SuiMoveValue::String("1000".into()))
            ])
        });

        let entry_ticket = SuiMoveValue::Variant(SuiMoveVariant {
            type_: new_structtag("0x1::game::EntryType")?,
            variant: "Ticket".into(),
            fields: BTreeMap::from([
                ("amount".into(), SuiMoveValue::String("500".into()))
            ])
        });

        // EntryLock test (assuming it's represented as a variant)
        let entry_locks = vec![
            SuiMoveValue::Variant(SuiMoveVariant {
                type_: new_structtag("0x1::game::EntryLock")?,
                variant: "Open".into(),
                fields: BTreeMap::<String, SuiMoveValue>::new()
            }),
            SuiMoveValue::Variant(SuiMoveVariant {
                type_: new_structtag("0x1::game::EntryLock")?,
                variant: "JoinOnly".into(),
                fields: BTreeMap::<String, SuiMoveValue>::new()
            }),
            SuiMoveValue::Variant(SuiMoveVariant {
                type_: new_structtag("0x1::game::EntryLock")?,
                variant: "DepositOnly".into(),
                fields: BTreeMap::<String, SuiMoveValue>::new()
            }),
            SuiMoveValue::Variant(SuiMoveVariant {
                type_: new_structtag("0x1::game::EntryLock")?,
                variant: "Closed".into(),
                fields: BTreeMap::<String, SuiMoveValue>::new()
            })
        ];

        // Test deserialization
        let players: Vec<PlayerJoin> = Vec::try_from_sui_move_value(&player_joins)?;
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].addr, addr1);
        assert_eq!(players[0].position, 1);

        let deposits: Vec<PlayerDeposit> = Vec::try_from_sui_move_value(&player_deposits)?;
        assert_eq!(deposits.len(), 1);
        assert_eq!(deposits[0].addr, addr1);
        assert_eq!(deposits[0].amount, 1000);

        let servers: Vec<ServerJoin> = Vec::try_from_sui_move_value(&server_joins)?;
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].addr, addr1);
        assert_eq!(servers[0].verify_key, "server_key1");

        let vote_list: Vec<Vote> = Vec::try_from_sui_move_value(&votes)?;
        assert_eq!(vote_list.len(), 1);
        assert_eq!(vote_list[0].voter, addr1);
        assert_eq!(vote_list[0].votee, addr2);

        // Test Options
        let some_addr: Option<SuiAddress> = Option::try_from_sui_move_value(&some_address)?;
        assert_eq!(some_addr, Some(addr1));
        let none_addr: Option<SuiAddress> = Option::try_from_sui_move_value(&none_address)?;
        assert_eq!(none_addr, None);

        let some_num: Option<u64> = Option::try_from_sui_move_value(&some_u64)?;
        assert_eq!(some_num, Some(100));
        let none_num: Option<u64> = Option::try_from_sui_move_value(&none_u64)?;
        assert_eq!(none_num, None);

        // Test EntryType
        // let cash_type = EntryType::try_from_sui_move_value(&entry_cash)?;
        // match cash_type {
        //     EntryType::Cash { min_deposit, max_deposit } => {
        //         assert_eq!(min_deposit, 100);
        //         assert_eq!(max_deposit, 1000);
        //     },
        //     _ => panic!("Wrong variant")
        // }

        let ticket_type = EntryType::try_from_sui_move_value(&entry_ticket)?;
        match ticket_type {
            EntryType::Ticket { amount } => {
                assert_eq!(amount, 500);
            },
            _ => panic!("Wrong variant")
        }

        // Test EntryLock
        for (i, lock_value) in entry_locks.iter().enumerate() {
            let lock = EntryLock::try_from_sui_move_value(lock_value)?;
            match i {
                0 => assert!(matches!(lock, EntryLock::Open)),
                1 => assert!(matches!(lock, EntryLock::JoinOnly)),
                2 => assert!(matches!(lock, EntryLock::DepositOnly)),
                3 => assert!(matches!(lock, EntryLock::Closed)),
                _ => panic!("Unexpected variant")
            }
        }

        // Test some error casesi: wrong types
        assert!(Vec::<PlayerJoin>::try_from_sui_move_value(&some_address).is_err());
        assert!(Option::<u64>::try_from_sui_move_value(&player_joins).is_err());
        assert!(EntryType::try_from_sui_move_value(&some_address).is_err());

        Ok(())
    }
}

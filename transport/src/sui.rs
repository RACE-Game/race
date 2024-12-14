#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
/// Transport for Sui blockchain
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use bcs;
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use serde::{Serialize, Deserialize};
use shared_crypto::intent::Intent;
use sui_config::{sui_config_dir, SUI_KEYSTORE_FILENAME};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_json_rpc_types::{Coin, CoinPage};
use sui_sdk::{
    rpc_types::{SuiMoveStruct, SuiObjectDataFilter, SuiObjectResponse,
                SuiObjectResponseQuery, SuiParsedData, SuiParsedMoveObject,
                SuiTransactionBlockResponseOptions
    },
    types::{
        TypeTag,
        base_types::{ObjectID, SuiAddress},
        crypto::{get_key_pair_from_rng, SuiKeyPair},
        programmable_transaction_builder::ProgrammableTransactionBuilder as PTB,
        quorum_driver_types::ExecuteTransactionRequestType, sui_serde::HexAccountAddress,
        transaction::{Argument, CallArg, Command, Transaction, TransactionData},
        Identifier
    },
    SuiClient, SuiClientBuilder, SUI_COIN_TYPE, SUI_DEVNET_URL
};
use tracing::{error, info, warn};

use std::{path::PathBuf, pin::Pin};
use std::str::FromStr;

use crate::error::{TransportError, TransportResult};
use race_core::{
    error::{Error, Result},
    transport::TransportT,
    types::{
        AssignRecipientParams, CloseGameAccountParams, CreateGameAccountParams,
        CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams,
        GameAccount, GameBundle, GameRegistration, JoinParams, PlayerProfile, PublishGameParams,
        RecipientAccount, RecipientClaimParams, RegisterGameParams, RegisterServerParams,
        RegistrationAccount, ServeParams, ServerAccount, SettleParams, SettleResult, Transfer, UnregisterGameParams, VoteParams, RecipientSlotInit, RecipientSlotShareInit, RecipientSlotType, RecipientSlotOwner
    }
};

// mods of this crate
mod constants;
mod types;
mod utils;
use constants::*;
use types::*;
use utils::*;

// Helper fns for interacting with Sui APIs

pub struct SuiTransport {
    // RPC node endpoint
    rpc: String,
    // on-chain package ID
    package_id: ObjectID,
    // active address associated with this transport, usually the `PUBLISHER`
    active_addr: SuiAddress,
    // TODO: use keypair
    keystore: FileBasedKeystore,
    client: SuiClient,
}

impl SuiTransport {
    async fn try_new(rpc: String, pkg_id: &str) -> TransportResult<Self> {
        println!("Create Sui transport at RPC: {} for packge: {:?}", rpc, pkg_id);
        let package_id = ObjectID::from_hex_literal(pkg_id)?;
        let active_addr = parse_str_addr(PUBLISHER)?;
        let keystore = FileBasedKeystore::new(&sui_config_dir()?.join(SUI_KEYSTORE_FILENAME))?;
        let client = SuiClientBuilder::default().build(rpc.clone()).await?;
        Ok(Self {
            rpc,
            package_id,
            active_addr,
            keystore,
            client
        })
    }

    fn get_package_id(&self) -> ObjectID {
        self.package_id.clone()
    }

    fn get_active_addr(&self) -> SuiAddress {
        self.active_addr.clone()
    }

    async fn get_coins(&self) -> TransportResult<Vec<Coin>> {
        let coin_page = self.client
            .coin_read_api()
            .get_coins(self.get_active_addr(), None, None, None)
            .await?;

        Ok(coin_page.data)
    }

    // generate a random address for some testing cases
    pub fn random_address() -> SuiAddress {
        SuiAddress::random_for_testing_only()
    }
}

#[async_trait]
impl TransportT for SuiTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        todo!()
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        todo!()
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        todo!()
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        todo!()
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        todo!()
    }

    // TODO: lowest priority
    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        todo!()
    }

    async fn create_recipient(&self, params: CreateRecipientParams) -> Result<String> {
        let mut used_ids = Vec::<u8>::new();
        let mut ptb = PTB::new();
        let module = new_identifier(RECIPIENT)?;
        let recipient_buider_fn = new_identifier(RECIPIENT_BUILDER_FN)?;
        let recipient_slot_fn = new_identifier(RECIPIENT_SLOT_FN)?;
        let slot_share_fn = new_identifier(SLOT_SHARE_FN)?;
        let recipient_fn = new_identifier(RECIPIENT_FN)?;

        // 1. move call new_recipient_builder to get a hot potato
        let builder = ptb.command(Command::move_call(
            self.get_package_id(),
            module.clone(),
            recipient_buider_fn.clone(),
            vec![],             // no type arguments,
            vec![]              // no arguments
        ));
        let mut final_builder = RecipientBuilderWrapper::new(builder.clone());

        // 2. a series of move calls to build recipient slots one by one
        for slot in params.slots.into_iter() {
            // slot id must be unique
            if used_ids.contains(&slot.id) {
                return Err(Error::InvalidRecipientSlotParams);
            }
            used_ids.push(slot.id);

            // 2.1. create shares for this slot and collect them into a vector
            let mut result_shares = Vec::new();
            for share in slot.init_shares.into_iter() {
                // prepare inputs for each share
                let (owner_type, owner_info) = match share.owner {
                    RecipientSlotOwner::Unassigned { identifier } => (0u8, identifier),
                    RecipientSlotOwner::Assigned { addr } => (1u8, addr),
                };
                let create_share_args = vec![
                    add_input(&mut ptb, &owner_type)?,
                    add_input(&mut ptb, &owner_info)?,
                    add_input(&mut ptb, &share.weights)?,
                ];

                let result = ptb.command(Command::move_call(
                    self.get_package_id(),
                    module.clone(),
                    slot_share_fn.clone(),
                    vec![],     // no T needed for shares
                    create_share_args
                ));

                result_shares.push(result);
            }

            // 2.2. add slot id, token_addr and slot type info
            let shares = ptb.command(Command::make_move_vec(
                Some(
                    TypeTag::Struct(Box::new(
                        StructTag {
                            address: AccountAddress::from_str(PACKAGE_ID)
                                .map_err(|e| Error::TransportError(e.to_string()))?,
                            module: new_identifier(RECIPIENT)?,
                            name: new_identifier(SLOT_SHARE_STRUCT)?,
                            type_params: vec![]
                        }
                    ))
                ),
                result_shares,
            ));

            let (coin_addr, coin_module, coin_name) = parse_coin_type(&slot.token_addr)?;
            let slot_type = match slot.slot_type {
                RecipientSlotType::Nft => 0u8,
                RecipientSlotType::Token => 1u8,
            };
            let build_slot_args = vec![
                add_input(&mut ptb, &slot.id)?,
                add_input(&mut ptb, &slot.token_addr)?,
                add_input(&mut ptb, &slot_type)?,
                shares,
                builder         // builder moved here in each loop
            ];

            let type_args = vec![
                TypeTag::Struct(Box::new(StructTag {
                    address: AccountAddress::from_str(&coin_addr)
                        .map_err(|e| Error::TransportError(e.to_string()))?,
                    module: new_identifier(&coin_module)?,
                    name: new_identifier(&coin_name)?,
                    type_params: vec![]
                }))
            ];

            // 2.3 move call to create the slot; return the hot potato for next loop
            let builder = ptb.command(Command::move_call(
                self.get_package_id(),
                module.clone(),
                recipient_slot_fn.clone(),
                type_args,         // Coin<T> for this slot
                build_slot_args,
            ));

            // store the updated builder result
            final_builder.update(builder.clone());

        }

        // 3. move call to create the recipient
        let cap_addr_arg: Option<SuiAddress> = parse_option_addr(params.cap_addr)?;
        let recipient_args = vec![
            add_input(&mut ptb, &cap_addr_arg)?,
            final_builder.builder()
        ];
        ptb.command(Command::move_call(
            self.get_package_id(),
            module.clone(),
            recipient_fn,
            vec![],             // no type arguments
            recipient_args,
        ));

        // 4. get coin for gas price, then sign, send and execute the transaction
        let coins = self.client
            .coin_read_api()
            .get_coins(self.get_active_addr(), None, None, None)
            .await?;
        let coin = coins.data.into_iter().next().unwrap();
        let gas_price = self.client.read_api().get_reference_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.get_active_addr(),
            vec![coin.object_ref()],
            ptb.finish(),
            GAS_BUDGET,
            gas_price,
        );

        let signature = self.keystore.sign_secure(
            &self.active_addr,
            &tx_data,
            Intent::sui_transaction(),
        )
            .map_err(|e| Error::TransportError(e.to_string()))?;

        let response = self.client
            .quorum_driver_api()
            .execute_transaction_block(
                Transaction::from_data(tx_data, vec![signature]),
                SuiTransactionBlockResponseOptions::new()
                    .with_effects()
                    .with_events(),
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;

        // TODO: return recipient object ID
        println!("Error (if any) {:?}", response.errors);
        Ok(response.digest.to_string())
    }

    async fn recipient_claim(&self, params: RecipientClaimParams) -> Result<()> {
        Ok(())
    }

    async fn assign_recipient(&self, params: AssignRecipientParams) -> Result<()> {
        Ok(())
    }

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        todo!()
    }

    async fn settle_game(&self, params: SettleParams) -> Result<SettleResult> {
        todo!()
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        todo!()
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        todo!()
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn get_game_account(&self, addr: &str) -> Result<Option<GameAccount>> {
        todo!()
    }

    async fn subscribe_game_account<'a>(&'a self, addr: &'a str) -> Result<Pin<Box<dyn Stream<Item = Result<GameAccount>> + Send + 'a>>> {
        todo!()
    }

    async fn get_game_bundle(&self, addr: &str) -> Result<Option<GameBundle>> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Result<Option<PlayerProfile>> {
        println!("Get player profile for {}", addr);
        let addr = SuiAddress::from_str(addr)
            .map_err(|e| Error::TransportError(e.to_string()))?;

        println!("Addr: {:?}", addr);
        let package = AccountAddress::from_str(PACKAGE_ID)
            .map_err(|e| Error::TransportError(e.to_string()))?;

        let filter_opts = Some(SuiObjectDataFilter::StructType(
            // xxxx::profile::PlayerProfile
            StructTag {
                address: package,
                module: Identifier::new("profile")
                    .map_err(|e| Error::TransportError(e.to_string()))?,
                name: Identifier::new("PlayerProfile")
                    .map_err(|e| Error::TransportError(e.to_string()))?,
                type_params: Default::default(),
            }
        ));
        let query = {
            Some(SuiObjectResponseQuery::new(
                filter_opts,
                None,
            ))
        };
        let data: Vec<SuiObjectResponse> = self.client.read_api().get_owned_objects(
            addr,
            query,
            None,
            None
        ).await.map_err(|e| Error::TransportError(e.to_string()))?.data;

        let content = data.first()
            .and_then(|first_item| first_item.data.clone())
            .and_then(|data| data.content)
            .ok_or(Error::PlayerProfileNotFound)?;

        let fields = match content {
            SuiParsedData::MoveObject(
                SuiParsedMoveObject {
                    fields: SuiMoveStruct::WithFields(fields) | SuiMoveStruct::WithTypes { fields, .. },
                    ..
                },
            ) => fields,
            _ => return Err(Error::PlayerProfileNotFound),
        };

        return Ok(Some(PlayerProfile {
            nick: fields.get("nick").map(|mv| mv.to_string()).unwrap_or("UNKNOWN".to_string()),
            pfp: fields.get("pfp").map(|mv| mv.to_string()),
            addr: addr.to_string(),
        }))
    }

    async fn get_server_account(&self, addr: &str) -> Result<Option<ServerAccount>> {
        todo!()
    }

    async fn get_registration(&self, addr: &str) -> Result<Option<RegistrationAccount>> {
        todo!()
    }

    async fn get_recipient(&self, addr: &str) -> Result<Option<RecipientAccount>> {
        todo!()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use anyhow;

    #[tokio::test]
    async fn test_get_player_profile() {
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), PACKAGE_ID).await.unwrap();
        let profile = transport.get_player_profile("0x13c43bafded256bdfda2e0fe086785aefa6e4ff45fb14fc3ca747b613aa12902").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_create_sui_transport() ->  TransportResult<()> {
        // let package_id = ObjectID::from_hex_literal(PACKAGE_ID)
        //     .map_err(|_| TransportError::ParseObjectIdError(PACKAGE_ID.into()))?;
        // let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), package_id).await?;
        // let address = transport.keystore.clone();
        // println!("Prepare to get balances");
        // let total_balance = transport.client
        //     .coin_read_api()
        //     .get_all_balances(address)
        //     .await?;
        // println!("The balances for all coins owned by address: {address} are {:?}", total_balance);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_recipient() -> TransportResult<()> {
        let params = CreateRecipientParams {
            cap_addr: Some("0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192".into()),
            slots: vec![
                RecipientSlotInit {
                    id: 0,
                    slot_type: RecipientSlotType::Token,
                    token_addr: COIN_SUI_ADDR.into(),
                    init_shares: vec![
                        RecipientSlotShareInit {
                            owner: RecipientSlotOwner::Unassigned {
                                identifier: "Race1".into()
                            },
                            weights: 10,
                        },
                        RecipientSlotShareInit {
                            owner: RecipientSlotOwner::Unassigned {
                                identifier: "Race2".into()
                            },
                            weights: 20,
                        }
                    ],
                }
            ]
        };
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), PACKAGE_ID).await?;

        let res = transport.create_recipient(params).await?;
        println!("Create recipient tx digest: {}", res);

        Ok(())
    }
}

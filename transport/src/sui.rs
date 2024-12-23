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
use sui_json_rpc_types::{
    Coin, CoinPage, SuiMoveValue, ObjectChange, SuiRawData, SuiRawMoveObject,
    SuiTransactionBlockResponse, SuiObjectDataOptions, SuiTransactionBlockEffectsAPI};
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
        transaction::{Argument, CallArg, Command, ProgrammableTransaction, Transaction, TransactionData},
        Identifier
    },
    SuiClient, SuiClientBuilder, SUI_COIN_TYPE, SUI_DEVNET_URL
};
use tracing::{error, info, warn};

use std::{path::PathBuf, pin::Pin};
use std::str::FromStr;
use std::collections::BTreeMap;
use crate::error::{TransportError, TransportResult};
use race_core::{
    error::{Error, Result},
    transport::TransportT,
    types::{
        AssignRecipientParams, CloseGameAccountParams, CreateGameAccountParams,
        CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams,
        GameAccount, GameBundle, GameRegistration, JoinParams, PlayerProfile, PublishGameParams, EntryType, EntryLock,
        RecipientAccount, RecipientClaimParams, RegisterGameParams, RegisterServerParams,
        RegistrationAccount, ServeParams, ServerAccount, SettleParams, SettleResult, Transfer, UnregisterGameParams, VoteParams,
        RecipientSlotInit, RecipientSlotShareInit, RecipientSlotType, RecipientSlotOwner
    }
};

// mods of this crate
mod constants;
mod types;
mod utils;
use constants::*;
use types::*;
use utils::*;

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

#[async_trait]
impl TransportT for SuiTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        if params.title.len() > MAX_GAME_NAME_LEN {
            return Err(TransportError::InvalidNameLength(params.title))?;
        }
        let payer = self.get_active_addr();
        let bundle_addr = parse_account_addr(&params.bundle_addr)?;
        let recipient_addr = parse_account_addr(&params.recipient_addr)?;
        let mut ptb = PTB::new();
        let module = new_identifier("game")?;
        let game_fn = new_identifier("create_game")?;
        let entry_type_arg: Argument = match params.entry_type {
            EntryType::Cash {min_deposit, max_deposit} => {
                let args = vec![
                    add_input(&mut ptb, &min_deposit)?,
                    add_input(&mut ptb, &max_deposit)?,
                ];
                let cmd_fn = new_identifier("create_cash_entry")?;
                ptb.command(self.make_command(module.clone(), cmd_fn, vec![], args))
            },
            EntryType::Ticket { amount } => {
                let args = vec![add_input(&mut ptb, &amount)?];
                let cmd_fn = new_identifier("create_ticket_entry")?;
                ptb.command(self.make_command(module.clone(), cmd_fn, vec![], args))
            },
            EntryType::Gating { collection } => {
                let args = vec![add_input(&mut ptb, &collection)?];
                let cmd_fn = new_identifier("create_gating_entry")?;
                ptb.command(self.make_command(module.clone(), cmd_fn, vec![], args))
            }
            EntryType::Disabled => {
                let cmd_fn = new_identifier("create_disabled_entry")?;
                ptb.command(self.make_command(module.clone(), cmd_fn, vec![], vec![]))
            }
        };
        let data_len: u32 = params.data.len().try_into()
            .map_err(|e: std::num::TryFromIntError|
                     Error::TransportError(e.to_string()))?;
        let create_game_args = vec![
            add_input(&mut ptb, & params.title)?,
            add_input(&mut ptb, &bundle_addr)?,
            add_input(&mut ptb, &payer)?,
            add_input(&mut ptb, &recipient_addr)?,
            add_input(&mut ptb, &params.token_addr)?,
            add_input(&mut ptb, &params.max_players)?,
            add_input(&mut ptb, &data_len)?,
            add_input(&mut ptb, &params.data)?,
            entry_type_arg
        ];

        let create_game_cmd = self.make_command(
            module.clone(),
            game_fn.clone(),
            vec![],
            create_game_args,
        );
        ptb.command(create_game_cmd);
        // get a ProgrammableTransaction type
        let pt = ptb.finish();

        // dry run to get the estimated total gas fee
        let coin = self.get_max_coin(Some(params.token_addr)).await?;
        let gas_price = self.get_gas_price().await?;
        let gas_fees = self.estimate_gas(gas_price, coin.clone(), pt.clone()).await?;

        println!("Needed gase fees {} and transport balance: {}",
                 gas_fees, coin.balance);

        // actually send the tx with the calcualted balance
        let tx_data = TransactionData::new_programmable(
            self.get_active_addr(),
            vec![coin.object_ref()],
            pt,
            coin.balance,
            gas_price
        );

        let response = self.send_transaction(tx_data).await?;

        println!("Tx digest: {}", response.digest.to_string());

        let object_changes: Vec<ObjectChange> = response.object_changes
            .unwrap_or_else(|| Vec::<ObjectChange>::new());

        if object_changes.len() == 0 {
            return Err(Error::TransportError("No game object created".into()));
        }

        let object_id: Option<String> = object_changes.iter()
            .find_map(|obj| match obj {
                ObjectChange::Created { object_id,  .. } => {
                    let obj_str_id: String = object_id.to_hex_uncompressed();
                    println!("Created registry object with id: {}", obj_str_id);
                    Some(obj_str_id)
                },
                _ => None
            });

        object_id.ok_or_else(|| Error::TransportError("No game created".into()))
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
                            address: parse_account_addr(PACKAGE_ID)?,
                            module: new_identifier(RECIPIENT)?,
                            name: new_identifier(SLOT_SHARE_STRUCT)?,
                            type_params: vec![]
                        }
                    ))
                ),
                result_shares,
            ));

            let (coin_addr, coin_module, coin_name) = parse_sui_path(&slot.token_addr)?;
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
                    address: parse_account_addr(&coin_addr)?,
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

        } // for slot ends

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
        // the coin type defaults to SUI
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
        ).map_err(|e| Error::TransportError(e.to_string()))?;

        let response = self.client
            .quorum_driver_api()
            .execute_transaction_block(
                Transaction::from_data(tx_data, vec![signature]),
                SuiTransactionBlockResponseOptions::new()
                    .with_effects()
                    .with_events()
                    .with_object_changes(),
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;

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
        let module = new_identifier("registry")?;
        let registry_fn = new_identifier("create_registry")?;
        let mut ptb = PTB::new();
        let registry_args = vec![
            add_input(&mut ptb, &params.is_private)?,
            add_input(&mut ptb, &params.size)?
        ];
        ptb.command(Command::move_call(
            self.get_package_id(),
            module,
            registry_fn,
            vec![],
            registry_args
        ));

        let pt = ptb.finish();
        let coin = self.get_max_coin(Some(COIN_SUI_ADDR.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let gas_fees = self.estimate_gas(gas_price, coin.clone(), pt.clone()).await?;
        let tx_data = TransactionData::new_programmable(
            self.get_active_addr(),
            vec![coin.object_ref()],
            pt,
            coin.balance,
            gas_price
        );

        let response = self.send_transaction(tx_data).await?;

        println!("Tx digest: {}", response.digest.to_string());

        let object_changes: Vec<ObjectChange> = response.object_changes
            .unwrap_or_else(|| Vec::<ObjectChange>::new());

        if object_changes.len() == 0 {
            return Err(Error::TransportError("No registry object created".into()));
        }

        let object_id: Option<String> = object_changes.iter()
            .find_map(|obj| match obj {
                ObjectChange::Created { object_id,  .. } => {
                    let obj_str_id: String = object_id.to_hex_uncompressed();
                    println!("Created registry object with id: {}", obj_str_id);
                    Some(obj_str_id)
                },
                _ => None
            });

        object_id.ok_or_else(|| Error::TransportError("No registry created".into()))
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        let game_id: ObjectID = parse_object_id(&params.game_addr)?;
        let registry_id: ObjectID = parse_object_id(&params.reg_addr)?;

        // get on chain game for title and bundle account address (may ID?)
        let game: GameObject = self.internal_get_game_object(game_id).await?;
        let module = new_identifier("registry")?;
        let reg_game_fn = new_identifier("register_game")?;
        let mut ptb = PTB::new();
        let reg_game_args: Vec<Argument> = vec![
        ];

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
        let addr = parse_sui_addr(addr)?;

        println!("Addr: {:?}", addr);
        let package = parse_account_addr(PACKAGE_ID)?;

        let filter_opts = Some(SuiObjectDataFilter::StructType(
            // xxxx::profile::PlayerProfile
            StructTag {
                address: package,
                module: new_identifier("profile")?,
                name: new_identifier("PlayerProfile")?,
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

impl SuiTransport {
    async fn try_new(rpc: String, pkg_id: &str) -> TransportResult<Self> {
        println!("Create Sui transport at RPC: {} for packge: {:?}", rpc, pkg_id);
        let package_id = ObjectID::from_hex_literal(pkg_id)?;
        let active_addr = parse_sui_addr(PUBLISHER)?;
        let keystore = FileBasedKeystore::new(
            &sui_config_dir()?.join(SUI_KEYSTORE_FILENAME)
        )?;
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

    // Get the coin with the most balance to pay the transaction gas fees.
    // The `String` in param `coin_type` represents a full Coin path, which defaults
    // to "0x2::sui::SUI" (if None is given)
    async fn get_max_coin(&self, coin_type: Option<String>) -> Result<Coin> {
        let coins: CoinPage = self.client
            .coin_read_api()
            .get_coins(self.get_active_addr(), coin_type, None, Some(50))
            .await?;
        coins.data.into_iter()
            .max_by_key(|c| c.balance)
            .ok_or(Error::TransportError("No Coin Found".to_string()))
    }

    async fn get_gas_price(&self) -> Result<u64> {
        Ok(self.client.read_api().get_reference_gas_price().await?)
    }

    // Get raw balance availble from all coins in the returned coin page
    async fn get_raw_balance(&self, coin_type: Option<String>) -> Result<u64> {
        let coin_page: CoinPage = self.client
            .coin_read_api()
            .get_coins(self.get_active_addr(), coin_type, None, Some(50))
            .await?;
        let balance = coin_page.data.into_iter().map(|c: Coin| c.balance).sum();
        Ok(balance)
    }

    async fn estimate_gas(
        &self,
        gas_price: u64,
        coin: Coin,
        pt: ProgrammableTransaction
    ) -> Result<u64> {
        let tx_data = TransactionData::new_programmable(
            self.get_active_addr(),
            vec![coin.object_ref()],
            pt,
            coin.balance,
            gas_price
        );

        // TODO: if dry run returns useful error message, print it
        let dry_run = self.client.read_api()
            .dry_run_transaction_block(tx_data)
            .await?;
        let cost_summary = dry_run.effects.gas_cost_summary();
        let net_gas_fees: i64 = cost_summary.net_gas_usage();
        println!("Got net gas fees: {} in MIST", net_gas_fees);

        if net_gas_fees < 0 {
            return Err(Error::TransportError("Unexpected negative gas fees".into()));
        };


        // add a small buffer to the estimated gas fees
        Ok(net_gas_fees as u64 + 50)
    }

    async fn send_transaction(
        &self,
        tx_data: TransactionData
    ) -> Result<SuiTransactionBlockResponse> {
        let sig = self.keystore.sign_secure(
            &self.active_addr,
            &tx_data,
            Intent::sui_transaction()
        ).map_err(|e| Error::TransportError(e.to_string()))?;

        // TODO: may need `with_balance_changes()`
        let response = self.client
            .quorum_driver_api()
            .execute_transaction_block(
                Transaction::from_data(tx_data, vec![sig]),
                SuiTransactionBlockResponseOptions::new()
                    .with_effects()
                    .with_events()
                    .with_object_changes(),
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;

        Ok(response)
    }

    // prepare a Command for move call
    fn make_command(
        &self,
        module: Identifier,
        fun: Identifier,
        type_args: Vec<TypeTag>,
        args: Vec<Argument>
    ) -> Command {
        Command::move_call(self.get_package_id(), module, fun, type_args, args)
    }

    // generate a random address for some testing cases
    fn rand_sui_addr() -> SuiAddress {
        SuiAddress::random_for_testing_only()
    }

    fn rand_sui_str_addr() -> String {
        SuiAddress::random_for_testing_only().to_string()
    }

    fn rand_account_addr() -> AccountAddress {
        AccountAddress::random()
    }

    fn rand_account_str_addr() -> String {
        AccountAddress::random().to_canonical_string(true)

    }

    // A few private helpers to query on chain objects, not for public uses
    async fn internal_get_game_object(
        &self,
        game_id: ObjectID
    ) -> Result<GameObject> {
        println!("Trying to get game object {}", game_id.to_hex_uncompressed());

        let response = self.client.read_api()
            .get_object_with_options(
                game_id,
                 SuiObjectDataOptions {
                    show_type: true,
                    show_owner: true,
                    show_previous_transaction: true,
                    show_display: true,
                    show_content: true,
                    show_bcs: true,
                    show_storage_rebate: true,
                }
            )
            .await?;


        let bcs: SuiRawData = response
            .data
            .and_then(|d| d.bcs)
            .ok_or(Error::GameAccountNotFound)?;

        let raw: SuiRawMoveObject = match bcs {
            SuiRawData::MoveObject(sui_raw_mv_obj) => {
                sui_raw_mv_obj
            },
            _ => return Err(Error::TransportError("TestEntryLock not found".into()))
        };

        println!("raw bytes: {:?}", raw.bcs_bytes);

        raw.deserialize().map_err(|e| Error::TransportError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PACKAGE_ID: &str = "0x6d9caadf9402936619002cd029a2334bddb5fabdfacb115768c8366105541e56";

    fn ser_game_obj_bytes() -> Result<Vec<u8>> {
        let game = GameObject {
            id: parse_object_id("0x22d111cb94424373a1873bfb770129b95b8ea5e609eed25b3750c20e9be2dff5")?,
            version: "0.1.0".to_string(),
            title: "Race Sui".to_string(),
            bundle_addr: parse_sui_addr("0xb4d6e06e2d8d76fd2c5340e17ff0d8e9de6be51be3a04d74c0fb66461435573e")?,
            coin_type: COIN_SUI_ADDR.to_string(),
            owner: parse_sui_addr(PUBLISHER)?,
            recipient_addr: parse_sui_addr("0xd37f3779435ee556815772daa05ceb93d00669b5f7c3cb89d81ec70fd70ad939")?,
            transactor_addr: None,
            access_version: 0,
            settle_version: 0,
            max_players: 6,
            players:vec![],
            deposits: vec![],
            servers:vec![],
            data_len: 5,
            data: vec![8,1,2,3,4],
            votes: vec![],
            unlock_time: None,
            entry_type: EntryType::Cash {min_deposit: 10, max_deposit: 100},
            checkpoint:vec![],
            entry_lock: EntryLock::Open,
        };
        let bytes = bcs::to_bytes(&game).map_err(|e| Error::TransportError(e.to_string()))?;
        println!("Original game bytes: {:?}", bytes);

        Ok(bytes)
    }

    #[tokio::test]
    async fn test_get_player_profile() {
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await.unwrap();
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
    async fn test_create_game() -> Result<()> {
        let params = CreateGameAccountParams {
            title: "Race Sui".into(),
            bundle_addr: SuiTransport::rand_account_str_addr(),
            token_addr: COIN_SUI_ADDR.into(),
            max_players: 6,
            entry_type: EntryType::Cash {min_deposit: 10, max_deposit: 100},
            recipient_addr: SuiTransport::rand_account_str_addr(),
            data: vec![8u8, 1u8, 2u8, 3u8, 4u8],
        };

        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;

        let digest = transport.create_game_account(params).await?;

        println!("Create game object tx digest: {}", digest);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_registration() -> Result<()> {
        let params = CreateRegistrationParams {
            is_private: false,
            size: 20
        };

        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;

        let object_id = transport.create_registration(params).await?;

        println!("Created registration object : {}", object_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_recipient() -> Result<()> {
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
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;

        let res = transport.create_recipient(params).await?;
        println!("Create recipient tx digest: {}", res);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_game_account() -> Result<()> {
        let game_id_str = "0x22d111cb94424373a1873bfb770129b95b8ea5e609eed25b3750c20e9be2dff5";
        let game_id = parse_object_id(game_id_str)?;
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;
        let game_obj = transport.internal_get_game_object(game_id).await?;

        assert_eq!(game_obj.title, "Race Sui".to_string());
        assert_eq!(game_obj.access_version, 0);
        assert_eq!(game_obj.settle_version, 0);
        assert_eq!(game_obj.max_players, 6);
        assert_eq!(game_obj.transactor_addr, None);
        assert_eq!(game_obj.players, vec![]);
        assert_eq!(game_obj.entry_type,
                   EntryType::Cash {min_deposit: 10, max_deposit: 100});
        assert_eq!(game_obj.entry_lock, EntryLock::Open);
        Ok(())
    }
}

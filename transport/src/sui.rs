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
use sui_types::object::Owner;
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
        base_types::{ObjectID, SuiAddress, SequenceNumber},
        crypto::{get_key_pair_from_rng, SuiKeyPair},
        programmable_transaction_builder::ProgrammableTransactionBuilder as PTB,
        quorum_driver_types::ExecuteTransactionRequestType, sui_serde::HexAccountAddress,
        transaction::{Argument, CallArg, Command, ObjectArg, ProgrammableTransaction, Transaction, TransactionData, },
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
        let payer = self.active_addr;
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
        // dry run to get the estimated total gas fee
        let pt = ptb.finish();
        let coin = self.get_max_coin(Some(params.token_addr)).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.active_addr,
            vec![coin.object_ref()],
            pt,
            coin.balance,
            gas_price
        );
        let gas_fees = self.estimate_gas(tx_data.clone()).await?;
        println!("Needed gase fees {} and transport has balance: {}",
                 gas_fees, coin.balance);

        // actually send the tx with the calcualted balance
        let response = self.send_transaction(tx_data).await?;

        println!("Creating game tx digest: {}", response.digest.to_string());

        let object_changes: Vec<ObjectChange> = response.object_changes
            .unwrap_or_else(|| Vec::<ObjectChange>::new());

        let game_id: Option<String> = object_changes.iter()
            .find_map(|obj| match obj {
                ObjectChange::Created { object_id,  .. } => {
                    let obj_str_id: String = object_id.to_hex_uncompressed();
                    println!("Created game object with id: {}", obj_str_id);
                    Some(obj_str_id)
                },
                _ => None
            });

        game_id.ok_or_else(|| Error::TransportError("No game created".into()))
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        todo!()
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        todo!()
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        println!("{} joining game {}", self.active_addr, params.game_addr);

        let module = new_identifier("game")?;
        let join_fn = new_identifier("join_game")?;
        let game_id = parse_object_id(&params.game_addr)?;
        let game_init_version = self.get_initial_shared_version(game_id).await?;
        let coin = self.get_max_coin(Some(COIN_SUI_ADDR.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            join_fn,
            vec![],             // no type arguments
            coin.object_ref(),
            vec![
                CallArg::Object(ObjectArg::SharedObject{
                    id: game_id,
                    initial_shared_version: game_init_version,
                    mutable: true
                }),
                new_callarg(&params.position)?,
                new_callarg(&params.access_version)?,
                new_callarg(&params.amount)?,
                new_callarg(&params.verify_key)?,
            ],
            coin.balance,
            gas_price,
        )?;

        let gas_fees = self.estimate_gas(tx_data.clone()).await?;
        println!("Needed gase fees {} and transport has balance: {}",
                         gas_fees, coin.balance);

        let response = self.send_transaction(tx_data).await?;

        println!("Joining game tx digest: {}", response.digest.to_string());

        Ok(())
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
        let mut used_ids: Vec<u8> = Vec::new();
        let module = new_identifier("recipient")?;
        let recipient_buider_fn = new_identifier("new_recipient_builder")?;
        let recipient_slot_fn = new_identifier("create_recipient_slot")?;
        let slot_share_fn = new_identifier("create_slot_share")?;
        let recipient_fn = new_identifier("create_recipient")?;
        let mut ptb = PTB::new();
        // 1. make move call to new_recipient_builder to get a hot potato
        let mut recipient_builder = ptb.command(Command::move_call(
            self.package_id,
            module.clone(),
            recipient_buider_fn,
            vec![],             // no type arguments,
            vec![]              // no arguments
        ));
        println!("RecipientBuilder starts as argument: {:?}", recipient_builder);

        // 2. make a series of move calls to build recipient slots one by one
        for slot in params.slots.into_iter() {
            // slot id must be unique
            if used_ids.contains(&slot.id) {
                println!("{:?} already contains slot id {}", used_ids, slot.id);
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
                    self.package_id,
                    module.clone(),
                    slot_share_fn.clone(),
                    vec![],     // no T needed for shares
                    create_share_args
                ));

                result_shares.push(result);
            }

            // 2.2. add slot id, token_addr and slot type info
            let shares = ptb.command(Command::make_move_vec(
                Some(TypeTag::Struct(Box::new(
                    StructTag {
                        address: parse_account_addr(&self.package_id.to_hex_uncompressed())?,
                        module: new_identifier("recipient")?,
                        name: new_identifier("RecipientSlotShare")?,
                        type_params: vec![]
                    }))),
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
                recipient_builder        // builder moved here in each loop on the chain as Result0
            ];
            let type_args = vec![
                TypeTag::Struct(Box::new(
                    StructTag {
                        address: parse_account_addr(&coin_addr)?,
                        module: new_identifier(&coin_module)?,
                        name: new_identifier(&coin_name)?,
                        type_params: vec![]
                    }))
            ];

            // 2.3 move call to create the slot; return the builder for next loop
            recipient_builder = ptb.command(Command::move_call(
                self.package_id,
                module.clone(),
                recipient_slot_fn.clone(),
                type_args,         // Coin<T> for this slot
                build_slot_args,
            ));
            println!("RecipientBuilder in-process as argument: {:?}", recipient_builder);

        } // for slot ends

        // 3. move call to create the recipient
        println!("Builder ends up as argument: {:?}", recipient_builder);

        let cap_addr_arg: Option<SuiAddress> = parse_option_addr(params.cap_addr)?;
        let recipient_args = vec![
            add_input(&mut ptb, &cap_addr_arg)?,
            recipient_builder
        ];
        ptb.command(Command::move_call(
            self.package_id,
            module.clone(),
            recipient_fn,
            vec![],             // no type arguments
            recipient_args,
        ));

        // 4. get max coin for gas price, then sign, send and execute the transaction
        let coin = self.get_max_coin(None).await?;
        let gas_price = self.client.read_api().get_reference_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.active_addr,
            vec![coin.object_ref()],
            ptb.finish(),
            coin.balance,
            gas_price,
        );

        let gas_fees = self.estimate_gas(tx_data.clone()).await?;

        println!("Needed gase fees {} and transport has balance: {}",
                 gas_fees, coin.balance);

        let response = self.send_transaction(tx_data).await?;
        println!("Creating recipient tx digest: {}", response.digest.to_string());

        // Search for `Recipient` struct among the created objects (many slots and one recipient )
        let identifier = new_identifier("Recipient")?;
        response.object_changes
            .and_then(|chs| chs.into_iter().find(|obj| match obj {
                &ObjectChange::Created { ref object_type, ..} => {
                    &object_type.name == &identifier
                },
                _ => false
            }))
            .and_then(|ch| {
                println!("Created recipient object with id {}", ch.object_id());
                Some(ch.object_id().to_hex_uncompressed())
            })
            .ok_or_else(|| Error::TransportError("No recipient created".into()))
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
        let registry_args = vec![
            new_callarg(&params.is_private)?,
            new_callarg(&params.size)?
        ];
        let coin = self.get_max_coin(Some(COIN_SUI_ADDR.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            registry_fn,
            vec![],             // no type arguments
            coin.object_ref(),
            registry_args,
            coin.balance,
            gas_price
        )?;

        let gas_fees = self.estimate_gas(tx_data.clone()).await?;

        println!("Needed gase fees {} and transport has balance: {}",
                 gas_fees, coin.balance);

        if gas_fees > coin.balance {
            return Err(Error::TransportError("insufficient balance for gas fees".into()));
        }

        let response = self.send_transaction(tx_data).await?;

        println!("Creating registry tx digest: {}", response.digest.to_string());

        let object_changes: Vec<ObjectChange> = response.object_changes
            .unwrap_or_else(|| Vec::<ObjectChange>::new());

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
        let game_id = parse_object_id(&params.game_addr)?;
        let registry_id = parse_object_id(&params.reg_addr)?;
        let clock_id = parse_object_id(CLOCK_ID)?;
        let game_version = self.get_initial_shared_version(game_id).await?;
        let registry_version = self.get_initial_shared_version(registry_id).await?;
        let clock_version = self.get_initial_shared_version(clock_id).await?;
        let module = new_identifier("registry")?;
        let reg_game_fn = new_identifier("register_game")?;
        let coin = self.get_max_coin(Some(COIN_SUI_ADDR.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            reg_game_fn,
            vec![],             // no type arguments
            coin.object_ref(),
            vec![
                CallArg::Object(ObjectArg::SharedObject{
                    id: game_id,
                    initial_shared_version: game_version,
                    mutable: false
                }),
                CallArg::Object(ObjectArg::SharedObject{
                    id: registry_id,
                    initial_shared_version: registry_version,
                    mutable: true
                }),
                CallArg::Object(ObjectArg::SharedObject{
                    id: clock_id,
                    initial_shared_version: clock_version, // always 1?
                    mutable: false
                })
            ],
            coin.balance,
            gas_price
        )?;

        let gas_fees = self.estimate_gas(tx_data.clone()).await?;

        println!("Needed gase fees {} and transport has balance: {}",
                 gas_fees, coin.balance);

        let response = self.send_transaction(tx_data).await?;

        println!("Registering game tx digest: {}", response.digest.to_string());

        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        println!("Unregistering game: {}", params.game_addr);
        println!("From registery: {}", params.reg_addr);

        let game_id = parse_object_id(&params.game_addr)?;
        let registry_id = parse_object_id(&params.reg_addr)?;
        let registry_version = self.get_initial_shared_version(registry_id).await?;
        let module = new_identifier("registry")?;
        let unreg_game_fn = new_identifier("unregister_game")?;
        let coin = self.get_max_coin(Some(COIN_SUI_ADDR.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            unreg_game_fn,
            vec![],             // no type arguments
            coin.object_ref(),
            vec![
                new_callarg(&game_id)?,
                CallArg::Object(ObjectArg::SharedObject{
                    id: registry_id,
                    initial_shared_version: registry_version,
                    mutable: true
                }),
            ],
            coin.balance,
            gas_price
        )?;

        let gas_fees = self.estimate_gas(tx_data.clone()).await?;
                println!("Needed gase fees {} and transport has balance: {}",
                 gas_fees, coin.balance);

        let response = self.send_transaction(tx_data).await?;

        println!("Unregistering game tx digest: {}", response.digest.to_string());

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

    // Get the coin with the most balance to pay the transaction gas fees.
    // The `String` in param `coin_type` represents a full Coin path, which defaults
    // to "0x2::sui::SUI" (if None is given)
    async fn get_max_coin(&self, coin_type: Option<String>) -> Result<Coin> {
        let coins: CoinPage = self.client
            .coin_read_api()
            .get_coins(self.active_addr, coin_type, None, Some(50))
            .await?;
        coins.data.into_iter()
            .max_by_key(|c| c.balance)
            .ok_or_else(|| Error::TransportError("No Coin Found".to_string()))
    }

    async fn get_gas_price(&self) -> Result<u64> {
        Ok(self.client.read_api().get_reference_gas_price().await?)
    }

    // Get raw balance availble from all coins in the returned coin page
    async fn get_raw_balance(&self, coin_type: Option<String>) -> Result<u64> {
        let coin_page: CoinPage = self.client
            .coin_read_api()
            .get_coins(self.active_addr, coin_type, None, Some(50))
            .await?;
        let balance = coin_page.data.into_iter().map(|c: Coin| c.balance).sum();
        Ok(balance)
    }

    async fn estimate_gas(&self, tx_data: TransactionData) -> Result<u64> {
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

    // The `initial_shared_version` is needed for mutating an on-chain object
    async fn get_initial_shared_version(&self, id: ObjectID) -> Result<SequenceNumber> {
        let response = self.client
            .read_api()
            .get_object_with_options(
                id,
                SuiObjectDataOptions::new().with_owner() // seqnum wrapped in `Owner`
            )
            .await?;

        response.data
            .and_then(|d| d.owner)
            .and_then(|o| match o {
                Owner::Shared { initial_shared_version } => {
                    println!("Initial sequm: {}", initial_shared_version.value());
                    Some(initial_shared_version)
                },
                _ => None
            })
            .ok_or_else(|| Error::TransportError("No initial shared version found".into()))
    }

    // Get the latest on-chain object sequencenumber
    async fn get_object_seqnum(&self, id: ObjectID) -> Result<SequenceNumber> {
        let response = self.client
            .read_api()
            .get_object_with_options(
                id,
                SuiObjectDataOptions::new()
            )
            .await?;

        let seqnum = response
            .data
            .and_then(|d| Some(d.version))
            .ok_or_else(|| Error::TransportError("No seuqeunce number found".into()))?;

        println!("Object {} with sequence number {}", id ,seqnum.value());

        Ok(seqnum)
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

        // NOTE: may need `with_balance_changes()`
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
        Command::move_call(self.package_id, module, fun, type_args, args)
    }

    // A few private helpers to query on chain objects, not for public uses
    async fn internal_get_game(
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
            SuiRawData::MoveObject(sui_raw_mv_obj) => sui_raw_mv_obj,
            _ => return Err(Error::TransportError("Game Object not found".into()))
        };

        // println!("raw bytes: {:?}", raw.bcs_bytes);

        raw.deserialize::<GameObject>()
            .map_err(|e| Error::TransportError(e.to_string()))
    }

    async fn internal_get_registry(
        &self,
        registry_id: ObjectID
    ) -> Result<RegistryObject> {
        println!("Trying to get registry object {:?}", registry_id);

        let response = self.client.read_api()
            .get_object_with_options(
                registry_id,
                SuiObjectDataOptions::bcs_lossless()
            )
            .await?;

        let raw_data: SuiRawData = response
            .data
            .and_then(|d| d.bcs)
            .ok_or_else(|| Error::RegistrationNotFound)?;

        let raw: SuiRawMoveObject = match raw_data {
            SuiRawData::MoveObject(sui_raw_mv_obj) => sui_raw_mv_obj,
            _ => return Err(Error::TransportError("Registry not found".into()))
        };

        raw.deserialize::<RegistryObject>()
            .map_err(|e| Error::TransportError(e.to_string()))
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

}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PACKAGE_ID: &str = "0xa443ad6e73d8ffbedbd25bf721698c7a9e7929d3838c4e5e849fd0eb7c4058fa";

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
    async fn test_get_seqnum() -> Result<()> {
        let game_id_str = "0x22d111cb94424373a1873bfb770129b95b8ea5e609eed25b3750c20e9be2dff5";
        let game_id = parse_object_id(game_id_str)?;
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;
        transport.get_object_seqnum(game_id).await?;

        Ok(())

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
            title: "Race Sui2".into(),
            bundle_addr: SuiTransport::rand_account_str_addr(),
            token_addr: COIN_SUI_ADDR.into(),
            max_players: 10,
            entry_type: EntryType::Ticket { amount: 20},
            recipient_addr: SuiTransport::rand_account_str_addr(),
            data: vec![8u8, 1u8, 2u8, 3u8, 4u8],
        };

        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;

        let game_id = transport.create_game_account(params).await?;

        println!("Create game object with id: {}", game_id);

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
                },
                RecipientSlotInit {
                    id: 1,
                    slot_type: RecipientSlotType::Nft,
                    token_addr: COIN_SUI_ADDR.into(),
                    init_shares: vec![
                        RecipientSlotShareInit {
                            owner: RecipientSlotOwner::Unassigned {
                                identifier: "RaceSui1".into()
                            },
                            weights: 20,
                        },
                        RecipientSlotShareInit {
                            owner: RecipientSlotOwner::Assigned {
                                addr: trim_prefix(PUBLISHER).to_string()
                            },
                            weights: 40,
                        }
                    ],
                }
            ]
        };
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;

        let res = transport.create_recipient(params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_register_game() -> Result<()> {
        let params = RegisterGameParams {
            game_addr: "0xaeeb09391060db21ac2699ccaf07ed51682441168c93ab5c7dfd498cd910871c".to_string(),
            reg_addr: "0x65f80e8f4e82f4885c96ccba4da02668428662e975b0a6cd1fa08b61e4e3a2fc".to_string()
        };
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;

        transport.register_game(params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_unregister_game() -> Result<()> {
        let params = UnregisterGameParams {
            game_addr: "0xaeeb09391060db21ac2699ccaf07ed51682441168c93ab5c7dfd498cd910871c".to_string(),
            reg_addr: "0x65f80e8f4e82f4885c96ccba4da02668428662e975b0a6cd1fa08b61e4e3a2fc".to_string()
        };

        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;

        transport.unregister_game(params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_join_game() -> Result<()> {
        let params = CreateGameAccountParams {
            title: "Race Sui Test".into(),
            bundle_addr: SuiTransport::rand_account_str_addr(),
            token_addr: COIN_SUI_ADDR.into(),
            max_players: 10,
            entry_type: EntryType::Cash { min_deposit: 20, max_deposit: 100 },
            recipient_addr: SuiTransport::rand_account_str_addr(),
            data: vec![8u8, 1u8, 2u8, 3u8, 4u8],
        };
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;
        let game_addr = transport.create_game_account(params).await?;
        let join_params = JoinParams {
            game_addr,
            access_version: 0,
            amount: 50,
            position: 2,
            verify_key: "player".to_string()
        };
        transport.join(join_params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_game_account() -> Result<()> {
        let game_id_str = "0x22d111cb94424373a1873bfb770129b95b8ea5e609eed25b3750c20e9be2dff5";
        let game_id = parse_object_id(game_id_str)?;
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), TEST_PACKAGE_ID).await?;
        let game_obj = transport.internal_get_game(game_id).await?;

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
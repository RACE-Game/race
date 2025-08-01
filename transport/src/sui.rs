#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
/// Transport for Sui blockchain
use async_stream::stream;
use async_trait::async_trait;
use bcs;
use borsh::BorshDeserialize;
use futures::{Stream, StreamExt};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use serde::{
    Serialize, Deserialize,
    de::DeserializeOwned,
};
use shared_crypto::intent::Intent;
use sui_config::{sui_config_dir, SUI_KEYSTORE_FILENAME};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_types::{
    base_types::ObjectRef,
    digests::{ObjectDigest, TransactionDigest},
    object::Owner,
};
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
use tokio::time;
use std::time::Duration;
use std::{path::PathBuf, pin::Pin};
use std::str::FromStr;
use std::collections::BTreeMap;
use crate::error::{TransportError, TransportResult};
use race_core::{
    checkpoint::CheckpointOnChain,
    error::{Error, Result},
    transport::TransportT,
    types::{
        AddRecipientSlotParams, AssignRecipientParams, Award, BalanceChange, CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams, EntryLock, EntryType, GameAccount, GameBundle, GameRegistration, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RecipientSlot, RecipientSlotInit, RecipientSlotOwner as CoreRecipientSlotOwner, RecipientSlotShareInit, RecipientSlotType, RegisterGameParams, RegisterServerParams, RegistrationAccount, RejectDepositsParams, RejectDepositsResult, ServeParams, ServerAccount, Settle, SettleParams, SettleResult, Transfer, UnregisterGameParams, VoteParams
    }
};

// mods of this crate
mod constants;
mod types;
mod utils;
mod nft;
pub use constants::*;
pub use types::*;
pub use utils::*;


pub struct SuiTransport {
    // RPC node endpoint
    rpc: String,
    // active address associated with this transport, usually the `PUBLISHER`
    active_addr: SuiAddress,
    // on-chain package ID
    package_id: ObjectID,
    // image url used for game bundle cover
    bundle_cover: String,
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
                    add_input(&mut ptb, new_pure_arg(&min_deposit)?)?,
                    add_input(&mut ptb, new_pure_arg(&max_deposit)?)?,
                ];
                let cmd_fn = new_identifier("create_cash_entry")?;
                ptb.command(self.make_command(module.clone(), cmd_fn, vec![], args))
            },
            EntryType::Ticket { amount } => {
                let args = vec![add_input(&mut ptb, new_pure_arg(&amount)?)?];
                let cmd_fn = new_identifier("create_ticket_entry")?;
                ptb.command(self.make_command(module.clone(), cmd_fn, vec![], args))
            },
            EntryType::Gating { collection } => {
                let args = vec![add_input(&mut ptb, new_pure_arg(&collection)?)?];
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
            add_input(&mut ptb, new_pure_arg(&params.title)?)?,
            add_input(&mut ptb, new_pure_arg(&bundle_addr)?)?,
            add_input(&mut ptb, new_pure_arg(&payer)?)?,
            add_input(&mut ptb, new_pure_arg(&recipient_addr)?)?,
            add_input(&mut ptb, new_pure_arg(&params.token_addr)?)?,
            add_input(&mut ptb, new_pure_arg(&params.max_players)?)?,
            add_input(&mut ptb, new_pure_arg(&data_len)?)?,
            add_input(&mut ptb, new_pure_arg(&params.data)?)?,
            entry_type_arg
        ];
        let(coin_addr, coin_module, coin_name) = parse_sui_path(&params.token_addr)?;
        let type_args = vec![
                TypeTag::Struct(Box::new(
                    StructTag {
                        address: parse_account_addr(&coin_addr)?,
                        module: new_identifier(&coin_module)?,
                        name: new_identifier(&coin_name)?,
                        type_params: vec![]
                    }))
        ];
        let create_game_cmd = self.make_command(
            module.clone(),
            game_fn.clone(),
            type_args,
            create_game_args,
        );
        ptb.command(create_game_cmd);
        let pt = ptb.finish();

        // dry run to get the estimated total gas fee
        let coin = self.get_max_coin(Some(params.token_addr)).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.active_addr,
            vec![coin.object_ref()],
            pt,
            coin.balance,
            gas_price
        );
        let _ = self.estimate_gas(tx_data.clone()).await?;

        // send the tx with the calcualted balance
        let response = self.send_transaction(tx_data).await?;

        println!("Creating game tx digest: {}", response.digest.to_string());

        let object_changes: Vec<ObjectChange> = response.object_changes
            .unwrap_or_else(Vec::<ObjectChange>::new);

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
        println!("Closing game: {}", params.addr);
        let module = new_identifier("game")?;
        let close_fn = new_identifier("close_game")?;
        let game_id = parse_object_id(&params.addr)?;
        let game_version = self.get_initial_shared_version(game_id).await?;
        let game_obj = self.get_move_object::<GameObject>(game_id).await?;

        if !game_obj.bonuses.is_empty() {
            return Err(Error::TransportError("Game bonuses not claimed".into()));
        }
        if game_obj.stake > 0 {
            return Err(Error::TransportError("Game stake is not 0".into()));
        }

        let gas_coin = self.get_max_coin(None).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            close_fn,
            vec![new_typetag(&game_obj.token_addr, None)?],
            gas_coin.object_ref(),
            vec![CallArg::Object(
                ObjectArg::SharedObject {
                    id: game_id,
                    initial_shared_version: game_version,
                    mutable: true
                })],
            gas_coin.balance,
            gas_price)?;
        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;
        println!("Closing game tx digest: {}", response.digest.to_string());

        Ok(())
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        println!("Registering server with endpoint {}", params.endpoint);

        //  Check if the transport already owns a server
        let owned: bool = self.check_owned_object("server::Server").await?;
        if owned {
            return Err(Error::TransportError("Already owned a server".into()));
        }

        let module = new_identifier("server")?;
        let reg_server_fn = new_identifier("register_server")?;
        let coin = self.get_max_coin(None).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            reg_server_fn,
            vec![],             // no type arguments
            coin.object_ref(),
            vec![new_pure_arg(&params.endpoint)?],
            coin.balance,
            gas_price)?;
        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;
        println!("Registering server tx digset: {}", response.digest.to_string());

        response.object_changes.map(|chs| {
            chs.iter().for_each(|obj| match obj {
                ObjectChange::Created { object_id, version, object_type, .. } => {
                    let server_path = self.get_canonical_path("server", "Server");
                    if object_type.to_canonical_string(true) == server_path {
                        println!("Created server: {object_id}");
                        println!("Server on-chain version: {version}");
                    }
                },
                _ => ()
            });
            chs             // make clippy happy as we need side effect only
        });

        Ok(())
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        println!("Player {} joining game {}", self.active_addr, params.game_addr);

        let module = new_identifier("game")?;
        let join_fn = new_identifier("join_game")?;
        let game_id = parse_object_id(&params.game_addr)?;
        let game_init_version = self.get_initial_shared_version(game_id).await?;
        let game_obj = self.get_move_object::<GameObject>(game_id).await?;
        let token_addr = game_obj.token_addr.clone();
        if game_obj.players.len() >= game_obj.max_players as usize {
            return Err(Error::TransportError("Game is already full".into()));
        }

        let mut ptb = PTB::new();

        // get player coins and merge them if there are more than one, then
        // use the merged one for both payment and gas fees
        let (arg_coins, gas_coins) = self.prepare_payment_coin(
            self.active_addr,
            token_addr.clone(),
            params.amount,
            &mut ptb
        ).await?;
        let arg_coins_vec = ptb.command(Command::make_move_vec(
            Some(new_typetag("0x2::coin::Coin", Some("0x2::sui::SUI"))?),
                arg_coins,
        ));

        // join game
        let join_args = vec![
            add_input(&mut ptb, new_obj_arg(ObjectArg::SharedObject {
                id: game_id,
                initial_shared_version: game_init_version,
                mutable: true
            })?)?,
            add_input(&mut ptb, new_pure_arg(&params.position)?)?,
            add_input(&mut ptb, new_pure_arg(&params.amount)?)?,
            add_input(&mut ptb, new_pure_arg(&params.verify_key)?)?,
            arg_coins_vec
        ];
        ptb.command(Command::move_call(
            self.package_id,
            module,
            join_fn,
            vec![new_typetag(&token_addr, None)?],
            join_args
        ));

        // let gas_coin = self.get_max_coin(Some(COIN_SUI_PATH.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.active_addr,
            gas_coins,
            ptb.finish(),
            5_000_000,
            gas_price
        );
        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;

        println!("Joining game tx digest: {}", response.digest.to_string());

        Ok(())
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    // `get_owned_object_ref` is used because each user or wallet addr can own
    // one and only one server on chain and there is no server ID info in params.
    // Another approaches would be: 1. include server addr info in the params;
    // 2. check on both ends to ensure the server is indeed owned by the sender
    async fn serve(&self, params: ServeParams) -> Result<()> {
        let module = new_identifier("game")?;
        let serve_fn = new_identifier("serve_game")?;
        let game_id = parse_object_id(&params.game_addr)?;
        let game_version = self.get_initial_shared_version(game_id).await?;
        let game_obj = self.get_move_object::<GameObject>(game_id).await?;

        if game_obj.servers.len() >= MAX_SERVER_NUM as usize {
            return Err(Error::TransportError("Game servers reaches the limit of 10".into()));
        }

        let server_obj_ref = self.get_owned_object_ref(
            self.active_addr,
            SuiObjectDataFilter::StructType(
                new_structtag(&format!("{}::server::Server", self.package_id), None)?
            )
        ).await?;
        let gas_coin = self.get_max_coin(None).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            serve_fn,
            vec![new_typetag(&game_obj.token_addr, None)?],
            gas_coin.object_ref(),
            vec![
                CallArg::Object(ObjectArg::SharedObject {
                    id: game_id,
                    initial_shared_version: game_version,
                    mutable: true
                }),
                CallArg::Object(ObjectArg::ImmOrOwnedObject(server_obj_ref)),
                new_pure_arg(&params.verify_key)?
            ],
            gas_coin.balance,
            gas_price
        )?;

        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;

        // Print all mutated objects including the game, server, and coin(sui)
        // Joining a game changes a server's version, even it passed as an immutable
        response.object_changes
            .map(|chs| {
                chs.iter().for_each(|obj| if let ObjectChange::Mutated {
                    object_id, version, previous_version, object_type, .. } = obj {
                    println!("Mutated object {} with id: {} ", object_type.name, object_id);
                    println!("Its version changed from {} to {}", version, previous_version);
                });
                chs             // return chs as is because we need side effects only
            });

        println!("Registering server tx digset: {}", response.digest.to_string());

        Ok(())
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
        let mut recipient_builder = ptb.programmable_move_call(
            self.package_id,
            module.clone(),
            recipient_buider_fn,
            vec![],             // no type arguments,
            vec![]              // no arguments
        );
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
                    CoreRecipientSlotOwner::Unassigned { identifier } => (0u8, identifier),
                    CoreRecipientSlotOwner::Assigned { addr } => (1u8, trim_prefix(&addr)),
                };

                let create_share_args = vec![
                    add_input(&mut ptb, new_pure_arg(&owner_type)?)?,
                    add_input(&mut ptb, new_pure_arg(&owner_info)?)?,
                    add_input(&mut ptb, new_pure_arg(&share.weights)?)?,
                ];

                let result = ptb.programmable_move_call(
                    self.package_id,
                    module.clone(),
                    slot_share_fn.clone(),
                    vec![],     // no T needed for shares
                    create_share_args
                );

                result_shares.push(result);
            }

            // 2.2. add slot id, token_addr and slot type info
            let path = format!(
                "{}::recipient::RecipientSlotShare",
                self.package_id,
            );
            let shares = ptb.command(Command::make_move_vec(
                Some(new_typetag(&path, None)?),
                result_shares,
            ));
            let slot_type = match slot.slot_type {
                RecipientSlotType::Nft => 0u8,
                RecipientSlotType::Token => 1u8,
            };
            let build_slot_args = vec![
                add_input(&mut ptb, new_pure_arg(&slot.id)?)?,
                add_input(&mut ptb, new_pure_arg(&slot.token_addr)?)?,
                add_input(&mut ptb, new_pure_arg(&slot_type)?)?,
                shares,
                recipient_builder        // builder moved here in each loop
            ];
            let type_args = vec![new_typetag(&slot.token_addr, None)?];

            // 2.3 move call to create the slot; return the builder for next loop
            recipient_builder = ptb.programmable_move_call(
                self.package_id,
                module.clone(),
                recipient_slot_fn.clone(),
                type_args,         // Coin<T> for this slot
                build_slot_args,
            );
            println!("RecipientBuilder in-process as argument: {:?}", recipient_builder);

        } // for slot ends

        // 3. move call to create the recipient
        println!("Builder ends up as argument: {:?}", recipient_builder);

        let cap_addr_arg: Option<SuiAddress> = parse_option_addr(params.cap_addr)?;
        let recipient_args = vec![
            add_input(&mut ptb, new_pure_arg(&cap_addr_arg)?)?,
            recipient_builder
        ];
        ptb.programmable_move_call(
            self.package_id,
            module.clone(),
            recipient_fn,
            vec![],             // no type arguments
            recipient_args,
        );

        // 4. get max coin for gas price, then sign, send and execute the transaction
        let gas_coin = self.get_max_coin(None).await?;
        let gas_price = self.client.read_api()
            .get_reference_gas_price()
            .await
            .map_err(|e| TransportError::GetGasPriceError(e.to_string()))?;

        let tx_data = TransactionData::new_programmable(
            self.active_addr,
            vec![gas_coin.object_ref()],
            ptb.finish(),
            gas_coin.balance,
            gas_price,
        );

        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;
        println!("Creating recipient tx digest: {}", response.digest.to_string());

        // Search for `Recipient` struct among the created objects (many slots and one recipient)
        let identifier = new_identifier("Recipient")?;
        response.object_changes
            .and_then(|chs| chs.into_iter().find(|obj| match obj {
                ObjectChange::Created { object_type, ..} => {
                    object_type.name == identifier
                },
                _ => false
            }))
            .map(|ch| {
                println!("Created recipient object: {}", ch.object_id());
                ch.object_id().to_hex_uncompressed() // return ID in string form
            })
            .ok_or_else(|| Error::TransportError("No recipient created".into()))
    }

    async fn add_recipient_slot(&self, params: AddRecipientSlotParams) -> Result<String> {
        unimplemented!()
    }

    async fn recipient_claim(&self, params: RecipientClaimParams) -> Result<()> {
        Ok(())
    }

    async fn assign_recipient(&self, params: AssignRecipientParams) -> Result<()> {
        Ok(())
    }

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        let module = new_identifier("game")?;
        let publish_fn = new_identifier("publish")?;
        let gas_coin = self.get_max_coin(None).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            publish_fn,
            vec![],             // no type argument
            gas_coin.object_ref(),
            vec![new_pure_arg(&params.name)?,
                 new_pure_arg(&params.symbol)?,
                 new_pure_arg(&params.uri)?,       // wasm bundle url
                 new_pure_arg(&self.bundle_cover)? // bundle cover image url
            ],
            gas_coin.balance,
            gas_price
        )?;
        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;
        println!("Publishing game tx digest: {}", response.digest.to_string());

        response.object_changes
            .and_then(|chs| chs.into_iter().find(|obj| match obj {
                ObjectChange::Created { .. } => true,
                _ => false
            }).map(|ch| {
                println!("Published game NFT: {}", ch.object_id());
                ch.object_id().to_hex_uncompressed()
            }))
            .ok_or_else(|| Error::TransportError("No game published".into()))
    }

    async fn settle_game(&self, params: SettleParams) -> Result<SettleResult> {
        let SettleParams {
            addr,
            settles,
            transfer,
            awards,
            checkpoint,
            access_version,
            settle_version,
            next_settle_version,
            entry_lock,
            accept_deposits
        } = params;
        if settles.len() + awards.len() + 10 > 1024 {
            return Err(Error::TransportError("Settles exceed the 1024 limit".into()));
        }
        let module = new_identifier("settle")?;
        let game_id = parse_object_id(&addr)?;
        let game_ref = self.get_object_ref(game_id).await?;
        let game_version = self.get_initial_shared_version(game_id).await?;
        let game_obj = self.get_move_object::<GameObject>(game_id).await?;
        let mut ptb = PTB::new();
        // run prechecks for settlemenet
        let game_obj_arg = ObjectArg::SharedObject {
            id: game_id,
            initial_shared_version: game_version,
            mutable: true
        };
        let pre_check_args = vec![
            add_input(&mut ptb, CallArg::Object(game_obj_arg))?,
            add_input(&mut ptb, new_pure_arg(&self.active_addr)?)?,
            add_input(&mut ptb, new_pure_arg(&settle_version)?)?,
            add_input(&mut ptb, new_pure_arg(&next_settle_version)?)?,
        ];
        // this returns the needed `checks_passed` input
        let checks_passed = ptb.programmable_move_call(
            self.package_id,
            module.clone(),
            new_identifier("pre_settle_checks")?,
            vec![new_typetag(&game_obj.token_addr, None)?],
            pre_check_args
        );
        // handle settles but skip when there is none
        if !settles.is_empty() {
            let mut result_settles: Vec<Argument> = Vec::new();
            for Settle { player_id, withdraw, eject, change } in settles {
                info!("Prepare settle for {}, amount = {}, eject: {}",
                         player_id, withdraw, eject);
                let (change_type, change_amt) = match change {
                    Some(bc) => {
                        match bc {
                            BalanceChange::Add(amt) => (1u8, amt),
                            BalanceChange::Sub(amt) => (2u8, amt),
                        }
                    },
                    None => (0u8, 0u64)
                };
                let args = vec![
                    add_input(&mut ptb, new_pure_arg(&player_id)?)?,
                    add_input(&mut ptb, new_pure_arg(&withdraw)?)?,
                    add_input(&mut ptb, new_pure_arg(&change_type)?)?,
                    add_input(&mut ptb, new_pure_arg(&change_amt)?)?,
                    add_input(&mut ptb, new_pure_arg(&eject)?)?,
                ];
                let settle_ret = ptb.programmable_move_call(
                    self.package_id,
                    module.clone(),
                    new_identifier("create_settle")?,
                    vec![],         // no type argument
                    args
                );
                result_settles.push(settle_ret);
            }

            let path = format!("{}::settle::Settle", self.package_id);
            let settles_vec = ptb.command(Command::make_move_vec(
                Some(new_typetag(&path, None)?),
                result_settles,
            ));
            let handle_settle_args = vec![
                add_input(&mut ptb, CallArg::Object(game_obj_arg))?,
                settles_vec,
                checks_passed
            ];
            ptb.programmable_move_call(
                self.package_id,
                module.clone(),
                new_identifier("handle_settles")?,
                vec![new_typetag(&game_obj.token_addr, None)?],
                handle_settle_args
            );
        }

        // prepare transfers
        let handle_transfer_fn = new_identifier("handle_transfer")?;
        let recipient_id = ObjectID::from_address(
            to_account_addr(game_obj.recipient_addr)?
        );
        let recipient_obj = self.get_move_object::<RecipientObject>(recipient_id).await?;
        let recipient_version = self.get_initial_shared_version(recipient_id).await?;
        // process transfers one by one
        if let Some(Transfer { amount }) = transfer {
            if let Some(slot) = recipient_obj.slots.iter().find(|s| s.token_addr.eq(&game_obj.token_addr)) {
                if game_obj.token_addr.ne(&slot.token_addr) {
                    return Err(Error::TransportError(format!(
                        "Expected token {} but got {}",
                        game_obj.token_addr, slot.token_addr)
                    ));
                }
                let slot_version = self.get_initial_shared_version(slot.id).await?;
                let handle_transfer_args = vec![
                    add_input(&mut ptb, CallArg::Object(game_obj_arg))?,
                    add_input(&mut ptb, CallArg::Object(ObjectArg::SharedObject{
                        id: slot.id,
                        initial_shared_version: slot_version,
                        mutable: true
                    }))?,
                    add_input(&mut ptb, CallArg::Object(ObjectArg::SharedObject{
                        id: recipient_id,
                        initial_shared_version: recipient_version,
                        mutable: true
                    }))?,
                    add_input(&mut ptb, new_pure_arg(&amount)?)?,
                    checks_passed
                ];
                ptb.programmable_move_call(
                    self.package_id,
                    module.clone(),
                    handle_transfer_fn.clone(),
                    vec![new_typetag(&game_obj.token_addr, None)?],
                    handle_transfer_args
                );
            } else {
                return Err(Error::InvalidSettle("Failed to find slot".to_string()));
            }
        }

        // handle awards one by one
        let handle_bonus_fn = new_identifier("handle_bonus")?;
        for Award {player_id, bonus_identifier} in awards.iter() {
            let Some(player) = game_obj.players
                .iter().find(|p| p.access_version == *player_id)
            else {
                return Err(Error::InvalidSettle(format!(
                    "Bonus not found for {} with identifier {}",
                    player_id, bonus_identifier
                )));
            };
            for Bonus {id, identifier, token_addr, amount} in game_obj.bonuses.iter() {
                if identifier.eq(bonus_identifier) {
                    let bonus_init_version = self.get_initial_shared_version(*id).await?;
                    let bonus_type_arg = if *amount == 0 { // NFT
                        let path = format!("{}::game::GameNFT", self.package_id);
                        vec![new_typetag(&path, None)?]
                    } else { // Coin
                        vec![new_typetag(COIN_TYPE_PATH, Some(token_addr))?]
                    };
                    let handle_bonus_args = vec![
                        add_input(&mut ptb, CallArg::Object(game_obj_arg))?,
                        add_input(&mut ptb, CallArg::Object(ObjectArg::SharedObject {
                            id: *id,
                            initial_shared_version: bonus_init_version,
                            mutable: true
                        }))?,
                        add_input(&mut ptb, new_pure_arg(&identifier)?)?,
                        add_input(&mut ptb, new_pure_arg(&player_id)?)?,
                        add_input(&mut ptb, new_pure_arg(&player.addr)?)?,
                        checks_passed
                    ];
                    ptb.programmable_move_call(
                        self.package_id,
                        module.clone(),
                        handle_bonus_fn.clone(),
                        bonus_type_arg,
                        handle_bonus_args
                    );
                }
            }
        }
        // update deposits, settle version, entry lock, etc
        let entry_lock_variant: u8 = match entry_lock {
            Some(lock) => match lock {
                EntryLock::Open => 0,
                EntryLock::JoinOnly => 1,
                EntryLock::DepositOnly => 2,
                EntryLock::Closed => 3,
            },
            None => 4
        };
        let entry_lock_create_arg = vec![add_input(&mut ptb, new_pure_arg(&entry_lock_variant)?)?];
        let entry_lock_arg = ptb.programmable_move_call(
            self.package_id,
            new_identifier("game")?,
            new_identifier("create_entry_lock")?,
            vec![],         // no type arg
            entry_lock_create_arg
        );

        let complete_settle_fn = new_identifier("finish_settle")?;
        let raw_checkpoint = borsh::to_vec(&checkpoint)?;
        let finish_args = vec![
            add_input(&mut ptb, CallArg::Object(game_obj_arg))?,
            add_input(&mut ptb, new_pure_arg(&accept_deposits)?)?,
            add_input(&mut ptb, new_pure_arg(&next_settle_version)?)?,
            add_input(&mut ptb, new_pure_arg(&raw_checkpoint)?)?,
            entry_lock_arg,
            checks_passed,
        ];
        ptb.programmable_move_call(
            self.package_id,
            module.clone(),
            complete_settle_fn,
            vec![new_typetag(&game_obj.token_addr, None)?],
            finish_args
        );

        // actually send the transaction
        let gas_coin = self.get_max_coin(None).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.active_addr,
            vec![gas_coin.object_ref()],
            ptb.finish(),
            100_000_000,
            gas_price
        );

        let response = self.send_transaction(tx_data).await?;
        let digest = response.digest;
        info!("Game settlement tx digest: {}", digest.to_string());

        let signature = response.digest.to_string();
        let status = self.confirm_settle_status(digest.clone())
            .await
            .map_err(|e| Error::TransportError(e.to_string()))?;

        if status  {
            let updated_game = self.get_move_object::<GameObject>(game_id).await?;
            let game_account = updated_game.into_account()?;

            return Ok(SettleResult {
                signature,
                game_account
            });
        } else {
            return Err(Error::TransportError(format!(
                "Settle transaction {} failed",
                digest.to_string()
            )));
        }
    }

    async fn reject_deposits(&self, params: RejectDepositsParams) -> Result<RejectDepositsResult> {
        let game_id = parse_object_id(&params.addr)?;
        let module = new_identifier("game")?;
        let reject_fn = new_identifier("reject_deposits")?;
        let game_version = self.get_initial_shared_version(game_id).await?;
        let game_obj = self.get_move_object::<GameObject>(game_id).await?;
        let gas_coin = self.get_max_coin(None).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            reject_fn,
            vec![new_typetag(&game_obj.token_addr, None)?],
            gas_coin.object_ref(),
            vec![
                CallArg::Object(ObjectArg::SharedObject {
                    id: game_id,
                    initial_shared_version: game_version,
                    mutable: true
                }),
                new_pure_arg(&params.reject_deposits)?
            ],
            gas_coin.balance,
            gas_price
        )?;

        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;
        println!("Rejecting desposits tx digest: {}", response.digest.to_string());

        response.object_changes
            .and_then(|chs| chs.iter().find(|ch| match ch {
                ObjectChange::Mutated { object_id, .. } => {
                    *object_id == game_id
                },
                _ => false
            }).and_then(|ch| {
                println!("Rejected deposits: {:?} from {game_id}", params.reject_deposits);
                Some(game_id)
            }))
            .ok_or_else(|| Error::TransportError("Failed to reject the deposits".into()))?;

        Ok(RejectDepositsResult {
            signature: response.digest.to_string()
        })
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        let module = new_identifier("registry")?;
        let registry_fn = new_identifier("create_registry")?;
        let registry_args = vec![
            new_pure_arg(&params.is_private)?,
            new_pure_arg(&params.size)?
        ];
        let coin = self.get_max_coin(Some(COIN_SUI_PATH.into())).await?;
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

        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;
        println!("Creating registry tx digest: {}", response.digest.to_string());

        let object_changes: Vec<ObjectChange> = response.object_changes
            .unwrap_or_else(Vec::<ObjectChange>::new);

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
        let module = new_identifier("registry")?;
        let reg_game_fn = new_identifier("register_game")?;
        let game_version = self.get_initial_shared_version(game_id).await?;
        let registry_version = self.get_initial_shared_version(registry_id).await?;
        let clock_version = self.get_initial_shared_version(clock_id).await?;
        let game_obj = self.get_move_object::<GameObject>(game_id).await?;
        let coin = self.get_max_coin(Some(COIN_SUI_PATH.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            reg_game_fn,
            vec![new_typetag(&game_obj.token_addr, None)?],
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

        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;
        // TODO: check the registry did get mutated or fail the tx otherwise
        println!("Registering game tx digest: {}", response.digest.to_string());

        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        info!("Unregistering game: {}", params.game_addr);
        info!("From registery: {}", params.reg_addr);

        let game_id = parse_object_id(&params.game_addr)?;
        let module = new_identifier("registry")?;
        let unreg_game_fn = new_identifier("unregister_game")?;
        let registry_id = parse_object_id(&params.reg_addr)?;
        let registry_version = self.get_initial_shared_version(registry_id).await?;
        let coin = self.get_max_coin(Some(COIN_SUI_PATH.into())).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_move_call(
            self.active_addr,
            self.package_id,
            module,
            unreg_game_fn,
            vec![],             // no type arguments
            coin.object_ref(),
            vec![new_pure_arg(&game_id)?,
                 new_obj_arg(ObjectArg::SharedObject{
                     id: registry_id,
                     initial_shared_version: registry_version,
                     mutable: true
                 })?,
            ],
            coin.balance,
            gas_price
        )?;

        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;

        info!("Unregistering game tx digest: {}", response.digest.to_string());

        Ok(())
    }

    async fn get_game_account(&self, addr: &str) -> Result<Option<GameAccount>> {
        let game_id = parse_object_id(addr)?;
        let game_obj = self.get_move_object::<GameObject>(game_id).await?;

        Ok(Some(game_obj.into_account()?))
    }

    async fn subscribe_game_account<'a>(&'a self, addr: &'a str) -> Result<Pin<Box<dyn Stream<Item = Result<GameAccount>> + Send + 'a>>> {
        Ok(Box::pin(stream! {
            let mut access_version = 0;
            loop {
                match self.get_game_account(addr).await {
                    Ok(game_account_opt) => {
                        if let Some(game_account) = game_account_opt {
                            if game_account.access_version > access_version {
                                access_version = game_account.access_version;
                                yield Ok(game_account);
                            }
                        }
                    }
                    Err(e) => ()
                }
                time::sleep(Duration::from_secs(8)).await;
            }
        }))
    }

    async fn get_game_bundle(&self, addr: &str) -> Result<Option<GameBundle>> {
        let bundle_id = parse_object_id(addr)?;
        let bundle_obj = self.get_move_object::<nft::BundleObject>(bundle_id).await?;
        let uri: &str = bundle_obj.uri.trim_end_matches('\0');

        info!("Fetch wasm game bundle from {}", uri);
        let data = nft::fetch_wasm_from_game_bundle(uri)
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))?;

        Ok(Some(bundle_obj.into_bundle(data)))
    }

    async fn get_player_profile(&self, addr: &str) -> Result<Option<PlayerProfile>> {
        info!("Get player profile for {}", addr);
        let addr = parse_sui_addr(addr)?;

        info!("Addr: {:?}", addr);

        let profile_ref = self.get_owned_object_ref(
            addr,
            SuiObjectDataFilter::StructType(
                new_structtag(&format!("{}::profile::PlayerProfile",
                                       self.package_id), None)?
            )
        ).await?;
        let profile_obj = self.get_move_object::<PlayerProfileObject>(profile_ref.0)
            .await?;

        Ok(Some(profile_obj.into_profile()))
    }

    // The param `address` is the one stored as `transactor_addr` in
    // the on-chain Game Object, thus the server should be one and only one server
    // object owned by this `address`
    async fn get_server_account(&self, addr: &str) -> Result<Option<ServerAccount>> {
        let server_ref = self.get_owned_object_ref(
            parse_sui_addr(addr)?,
            SuiObjectDataFilter::StructType(
                new_structtag(&format!("{}::server::Server", self.package_id), None)?
            )
        ).await?;
        let server_obj = self.get_move_object::<ServerObject>(server_ref.0).await?;

        Ok(Some(server_obj.into_account()))
    }

    async fn get_registration(&self, addr: &str) -> Result<Option<RegistrationAccount>> {
        let reg_id = parse_object_id(addr)?;
        let reg_obj = self.get_move_object::<RegistryObject>(reg_id).await?;
        Ok(Some(reg_obj.into_account()))
    }

    async fn get_recipient(&self, addr: &str) -> Result<Option<RecipientAccount>> {
        let recipient_id = parse_object_id(addr)?;
        let recipient_obj = self.get_move_object::<RecipientObject>(recipient_id).await?;

        info!("Recipient has {} slots", recipient_obj.slots.len());

        Ok(Some(recipient_obj.into()))
    }
}

impl SuiTransport {
    pub async fn try_new(
        rpc: String,
        pkg_id: &str,
        keyfile: Option<PathBuf>
    ) -> TransportResult<Self> {
        info!("Create Sui transport at RPC: {} for packge: {:?}", rpc, pkg_id);
        let package_id = parse_object_id(pkg_id)?;
        let active_addr = parse_sui_addr(PUBLISHER)?;

        let keystore = FileBasedKeystore::new(
            &keyfile.unwrap_or(sui_config_dir()?.join(SUI_KEYSTORE_FILENAME))
        )?;
        let client = SuiClientBuilder::default().build(rpc.clone()).await?;
        Ok(Self {
            rpc,
            active_addr,
            package_id,
            bundle_cover: BUNDLE_COVER.to_string(),
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
            .await
            .map_err(|e| TransportError::GetBalanceError(e.to_string()))?;
        coins.data.into_iter()
            .max_by_key(|c| c.balance)
            .ok_or_else(|| Error::TransportError("No max coin found".to_string()))
    }

    // Get up to 50 coins and check if the total balance is equal to or bigger than
    // the given amount and abort upon any errors.  If everything goes well, the
    // coins will be categorized into two groups: one as `Argument` to be passed to
    // the `join_game` move call; the other to be used as gas payments. The second
    // return value must contain at least one coin reference
    async fn prepare_payment_coin(
        &self,
        addr: SuiAddress,
        coin_type: String,
        amount: u64,
        ptb: &mut PTB,
    ) -> Result<(Vec<Argument>, Vec<ObjectRef>)> {
        let coin_page: CoinPage = self.client
            .coin_read_api()
            .get_coins(addr, Some(coin_type.clone()), None, Some(50))
            .await
            .map_err(|e| TransportError::GetBalanceError(e.to_string()))?;

        if coin_page.data.is_empty() {
            return Err(Error::TransportError(
                format!("Failed to get coins from {}", addr))
            );
        }
        let coins = coin_page.data;
        // add a small buffer roughly equal to the estimated net gas for simple tx
        // see https://docs.sui.io/concepts/tokenomics/gas-in-sui#gas-budget-examples
        let total_balance = coins.iter().fold(0u64, |acc, c| acc + c.balance);
        if total_balance < amount + 500_000u64 {
            return Err(Error::TransportError(
                format!("{} has insufficient balance for payment and gas", addr))
            );
        }

        // coins used for payment (buyin, bouns, etc)
        let mut arg_coins: Vec<Argument> = Vec::new();
        let mut gas_coins: Vec<ObjectRef> = Vec::new();
        let mut payment: u64 = 0;
        let mut gas_start: usize = 0;
        for (i, coin) in coins.iter().enumerate() {
            payment += coin.balance;
            if payment >= amount {
                let amt = coin.balance - (payment - amount);
                let split_amt_arg = vec![add_input(ptb, new_pure_arg(&amt)?)?];
                ptb.command(Command::SplitCoins(
                    Argument::GasCoin,
                    split_amt_arg
                ));
                // record the this coin ref in gas
                gas_coins.push(coin.object_ref());
                arg_coins.push(Argument::NestedResult(0,0));
                gas_start = i + 1;
                break;
            } else {
                arg_coins.push(add_input(
                    ptb,
                    new_obj_arg(ObjectArg::ImmOrOwnedObject(coin.object_ref()))?
                )?);
            }
        }

        coins.iter().skip(gas_start).for_each(|c| gas_coins.push(c.object_ref()));
        Ok((arg_coins, gas_coins))
    }

    async fn get_gas_price(&self) -> Result<u64> {
        Ok(self.client.read_api()
           .get_reference_gas_price()
           .await
           .map_err(|e| TransportError::GetGasPriceError(e.to_string()))?
        )
    }

    async fn estimate_gas(&self, tx_data: TransactionData) -> Result<u64> {
        let dry_run = self.client.read_api()
            .dry_run_transaction_block(tx_data)
            .await
            .map_err(|e| TransportError::GetGasPriceError(e.to_string()))?;
        let cost_summary = dry_run.effects.gas_cost_summary();
        let net_gas_fees: i64 = cost_summary.net_gas_usage();
        info!("Net gas fees: {} MIST", net_gas_fees);

        if net_gas_fees < 0 {
            info!("Tx sender will get rebate: {} MIST", -net_gas_fees);
            Ok(0)
        } else {
            // add a small buffer to the estimated gas fees
            Ok(net_gas_fees as u64 + 50)
        }
    }

    // The `initial_shared_version` is needed for mutating an on-chain object
    async fn get_initial_shared_version(&self, id: ObjectID) -> Result<SequenceNumber> {
        let response = self.client
            .read_api()
            .get_object_with_options(
                id,
                SuiObjectDataOptions::new().with_owner() // seqnum wrapped in `Owner`
            )
            .await
            .map_err(|e| TransportError::GetVersionError(e.to_string()))?;

        response.data
            .and_then(|d| d.owner)
            .and_then(|o| match o {
                Owner::Shared { initial_shared_version } => {
                    Some(initial_shared_version)
                },
                _ => None
            })
            .ok_or_else(|| Error::TransportError(format!(
                "No initial shared version found for {}", id)
            ))
    }

    // TODO: drop this function
    // Get the latest on-chain object sequencenumber
    async fn get_object_seqnum(&self, id: ObjectID) -> Result<SequenceNumber> {
        let response = self.client
            .read_api()
            .get_object_with_options(
                id,
                SuiObjectDataOptions::new()
            )
            .await
            .map_err(|e| TransportError::GetVersionError(e.to_string()))?;

        let seqnum = response
            .data
            .map(|d| d.version)
            .ok_or_else(|| Error::TransportError("No seuqeunce number found".into()))?;

        println!("Object {} with sequence number {}", id ,seqnum.value());

        Ok(seqnum)
    }

    // Get object id and initial shared version of a specific address-owned object
    // Used when the object id is unknown
    async fn get_owned_object_ref(
        &self,
        owner: SuiAddress,
        filter: SuiObjectDataFilter
    ) -> Result<ObjectRef> {
        let query = Some(SuiObjectResponseQuery::new(
                Some(filter),
                Some(SuiObjectDataOptions::new().with_owner())
        ));

        let data: Vec<SuiObjectResponse> = self.client
            .read_api()
            .get_owned_objects(
                owner,
                query,
                None,           // cursor
                Some(1)         // limit
            ).await.map_err(|e| Error::TransportError(e.to_string()))?
            .data;

        println!("Got reponses data {:?}", data[0]);

        data.first()
            .and_then(|first_item| first_item.data.clone())
            .map(|data| {
                let version = data.owner.and_then(|o| match o {
                    Owner::AddressOwner(_) => Some(data.version),
                    _ => None
                }).ok_or_else(|| Error::TransportError("init version not found".into()))?;
                Ok((data.object_id, version, data.digest))
            })                  // Some(Ok((id, v)))
            .transpose()        // Ok(Some((id, v)))
            .and_then(|ret| ret.ok_or_else(|| Error::TransportError("Queried owned object not found".into())))
    }

    /// Check if there is already one such owned object
    async fn check_owned_object(&self, object_path: &str) -> Result<bool> {
        let filter = SuiObjectDataFilter::StructType(
            new_structtag(&format!("{}::{}", self.package_id, object_path), None)?
        );

        let query = Some(SuiObjectResponseQuery::new(
            Some(filter),
            Some(SuiObjectDataOptions::new().with_owner())
        ));

        let data: Vec<SuiObjectResponse> = self.client
            .read_api()
            .get_owned_objects(
                self.active_addr,
                query,
                None,
                None
            ).await.map_err(|e| Error::TransportError(e.to_string()))?
            .data;

        Ok(!data.is_empty())
    }

    // Get the object ref for a shared object
    async fn get_object_ref(&self, object_id: ObjectID) -> Result<ObjectRef> {
        let response = self.client
            .read_api()
            .get_object_with_options(object_id, SuiObjectDataOptions::new())
            .await
            .map_err(|e| TransportError::GetObjectError(e.to_string()))?;

        response
            .object_ref_if_exists()
            .ok_or_else(|| Error::TransportError("ObjectRef not found".into()))
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
            .await
            .map_err(|e| TransportError::ClientSendTransactionFailed(e.to_string()))?;

        Ok(response)
    }

    // confirm the status of a settlement transaction
    async fn confirm_settle_status(
        &self,
        digest: TransactionDigest
    ) -> TransportResult<bool> {
        const MAX_RETRIES: u8 = 30;
        let mut i = 0u8;
        while i < MAX_RETRIES {
            let tx_resp = self.client.read_api().get_transaction_with_options(
                digest,
                SuiTransactionBlockResponseOptions::new().with_effects()
            ).await?;
            if tx_resp.status_ok().unwrap_or(false) {
                return Ok(true);
            }
            i += 1;
            // sleep here
            time::sleep(Duration::from_millis(500)).await;
        }
        Ok(false)
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

    // attach a bonus to the given game
    async fn attach_bonus(
        &self,
        params: AttachBonusParams
    ) -> Result<()> {
        let module = new_identifier("game")?;
        let mut ptb = PTB::new();
        let bonus_id: Argument = match params.bonus_type {
            BonusType::Coin(coin_type) => {
                // get and prepare the coin used for bonus
                let palyer_coins_arg = self.prepare_payment_coin(
                    self.active_addr,
                    coin_type.clone(),
                    params.amount,
                    &mut ptb
                ).await?;

                // acutally create the coin bonus
                let coin_bonus_fn = new_identifier("create_coin_bonus")?;
                let args = vec![
                    add_input(&mut ptb, new_pure_arg(&params.identifier)?)?,
                    add_input(&mut ptb, new_pure_arg(&coin_type)?)?,
                    add_input(&mut ptb, new_pure_arg(&params.amount)?)?,
                    Argument::NestedResult(0,0) // coin arg
                ];
                ptb.programmable_move_call(
                    self.package_id,
                    module.clone(),
                    coin_bonus_fn,
                    vec![new_typetag(COIN_TYPE_PATH, Some(&coin_type))?], // Coin<T>
                    args
                )
            },
            BonusType::Object(obj_id) => {
                let obj_bonus_fn = new_identifier("create_object_bonus")?;
                let obj_path = format!(
                    "{}::{}::{}",
                    &self.package_id, "game", "GameNFT"
                );
                let obj_ref = self.get_object_ref(obj_id).await?;
                let args = vec![
                    add_input(&mut ptb, new_pure_arg(&params.identifier)?)?,
                    add_input(
                        &mut ptb,
                        new_obj_arg(ObjectArg::ImmOrOwnedObject(obj_ref))?
                    )?
                ];
                ptb.programmable_move_call(
                    self.package_id,
                    module.clone(),
                    obj_bonus_fn,
                    vec![new_typetag(&obj_path, None)?],
                    args
                )
            }
        };

        // attach bonus
        let attach_fn = new_identifier("attach_bonus")?;
        let game_version = self.get_initial_shared_version(params.game_id).await?;
        let args = vec![
            add_input(&mut ptb, new_obj_arg(ObjectArg::SharedObject {
                id: params.game_id,
                initial_shared_version: game_version,
                mutable: true
            })?)?,
            bonus_id
        ];
        ptb.programmable_move_call(
            self.package_id,
            module,
            attach_fn,
            vec![new_typetag(&params.token_addr, None)?],
            args
        );
        // transaction
        let gas_coin = self.get_max_coin(None).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.active_addr,
            vec![gas_coin.object_ref()],
            ptb.finish(),
            gas_coin.balance,
            gas_price
        );
        let _ = self.estimate_gas(tx_data.clone()).await?;
        let response = self.send_transaction(tx_data).await?;

        println!("Attaching bonus tx digest: {}", response.digest.to_string());
        // game object changed and bonus object created
        response.object_changes
            .and_then(|chs| {
                let collected: Vec<_> = chs.into_iter()
                    .filter_map(|obj| match obj {
                        ObjectChange::Created { object_id, .. } => {
                            println!("Created bonus object: {object_id}");
                            Some(*object_id)
                        },
                        ObjectChange::Mutated { object_id, .. } => {
                            if *object_id == *params.game_id {
                                println!("Bonus attached to game: {object_id}");
                                Some(*object_id)
                            } else {
                                None
                            }
                        },
                        _ => None
                    })
                    .collect();

                if collected.len() >= 2 {
                    Some(collected)
                } else {
                    None
                }
            })
            .ok_or_else(|| Error::TransportError("Expected 2 obj changes".into()))?;

        Ok(())
    }

    // get on chain move objects and deserialize it into corresponding off chain struct
    async fn get_move_object<T: DeserializeOwned>(
        &self,
        object_id: ObjectID
    ) -> Result<T> {
        let raw = self.client.read_api()
            .get_move_object_bcs(object_id)
            .await
            .map_err(|e| TransportError::GetObjectError(e.to_string()))?;
        // println!("{:?}", raw);
        bcs::from_bytes::<T>(raw.as_slice())
            .map_err(|e| Error::TransportError(e.to_string()))
    }

    // get a canonical string representation of the format: 0xpackage_id::module::name
    fn get_canonical_path(&self, module: &str, name: &str) -> String {
        format!("{}::{}::{}", &self.package_id, module, name)
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

    // temporary IDs for quick tests
    const TEST_PACKAGE_ID: &str = "0x1d69af8651c81c19eeca3411f276177f3627ffb5a3da6851a3f9257f210f3d4b";
    const TEST_CASH_GAME_ID: &str = "0x6ba1817f72aea249b6d1ca5bf01fceef91e2704e944141acfc7013f129a90847";
    const TEST_TICKET_GAME_ID: &str = "0xcfc82be4212e504a2bc8b9a6b5b66ed0db92be4e2ab0befe5ba7146a59f54665";
    const TEST_RECIPIENT_ID: &str = "0x8b8e76d661080e47d76248cc33b43324b4126a8532d7642ab6c47946857c1e1c";
    const TEST_REGISTRY: &str = "0x6f819d1497313b8e059f6abc29ce726590c2c5a0f4b86497fee344cf0a6810d6";
    const TEST_GAME_NFT: &str = "0x5ebed419309e71c1cd28a3249bbf792d2f2cc8b94b0e21e45a9873642c0a5cdc";

    // helper fns to generate some large structures for tests
    fn make_game_params() -> CreateGameAccountParams {
        // update entry type if needed
        let entry_type = EntryType::Cash { max_deposit: 200_000_000,
                                           min_deposit: 100_000_000 }; // 0.1 SUI
        CreateGameAccountParams {
            title: "Race Devnet Test".into(),
            bundle_addr: SuiTransport::rand_account_str_addr(),
            token_addr: COIN_SUI_PATH.into(),
            max_players: 9,
            entry_type,
            recipient_addr: TEST_RECIPIENT_ID.to_string(),
            data: vec![128,150,152,0,0,0,0,0,0,45,49,1,0,0,0,0,0,0,0,0,0,0,0,0,30,0,1,0,94,208,178,0,0,0,0,0],
        }
    }

    fn make_recipient_params() -> CreateRecipientParams {
        CreateRecipientParams {
            cap_addr: Some(PUBLISHER.into()),
            slots: vec![
                RecipientSlotInit {
                    id: 0,
                    slot_type: RecipientSlotType::Token,
                    token_addr: COIN_SUI_PATH.into(),
                    init_shares: vec![
                        RecipientSlotShareInit {
                            owner: CoreRecipientSlotOwner::Unassigned {
                                identifier: "Race1".into()
                            },
                            weights: 10,
                        },
                        RecipientSlotShareInit {
                            owner: CoreRecipientSlotOwner::Unassigned {
                                identifier: "Race2".into()
                            },
                            weights: 20,
                        }
                    ],
                },
                RecipientSlotInit {
                    id: 1,
                    slot_type: RecipientSlotType::Nft,
                    token_addr: COIN_SUI_PATH.into(),
                    init_shares: vec![
                        RecipientSlotShareInit {
                            owner: CoreRecipientSlotOwner::Unassigned {
                                identifier: "RaceSui1".into()
                            },
                            weights: 20,
                        },
                        RecipientSlotShareInit {
                            owner: CoreRecipientSlotOwner::Assigned {
                                addr: PUBLISHER.to_string()
                            },
                            weights: 40,
                        }
                    ],
                }
            ]
        }
    }

    #[test]
    fn ser_game_obj() -> Result<()> {
        let game = GameObject {
            id: parse_object_id(TEST_CASH_GAME_ID)?,
            version: "0.1.0".to_string(),
            title: "Race Devnet Test".to_string(),
            bundle_addr: parse_sui_addr(TEST_GAME_NFT)?,
            token_addr: COIN_SUI_PATH.to_string(),
            owner: parse_sui_addr(PUBLISHER)?,
            recipient_addr: parse_sui_addr(TEST_RECIPIENT_ID)?,
            transactor_addr: None,
            access_version: 0,
            settle_version: 0,
            max_players: 10,
            players: vec![ PlayerJoin {
                addr: parse_sui_addr(PUBLISHER)?,
                position: 2u16,
                access_version: 1,
                verify_key: "player".to_string()
            }],
            deposits: vec![ PlayerDeposit {
                addr: parse_sui_addr(PUBLISHER)?,
                amount: 100,
                access_version: 1,
                settle_version: 0,
                status: DepositStatus::Accepted
            }],
            stake: 0,
            servers: vec![],
            balances: vec![],
            data_len: 5,
            data: vec![8,1,2,3,4],
            votes: vec![],
            unlock_time: None,
            entry_type: EntryType::Ticket { amount: 100 },
            checkpoint:vec![],
            entry_lock: EntryLock::Open,
            bonuses: vec![]
        };
        let bytes = bcs::to_bytes(&game).map_err(|e| Error::TransportError(e.to_string()))?;
        println!("Original game bytes: {:?}", bytes);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_player_profile() {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let profile = transport.get_player_profile(PUBLISHER).await;
    }

    #[tokio::test]
    async fn test_get_seqnum() -> Result<()> {
        let game_id = parse_object_id(TEST_CASH_GAME_ID)?;
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        transport.get_object_seqnum(game_id).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_create_recipient() -> Result<()> {
        let params = make_recipient_params();
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();

        let res = transport.create_recipient(params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_recipient_object() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        // get the recipient object
        let recipient_id = parse_object_id(TEST_RECIPIENT_ID)?;
        let recipient_obj = transport.get_move_object::<RecipientObject>(recipient_id).await?;
        let cap_addr = parse_sui_addr(PUBLISHER)?;
        println!("Found recipient {}", recipient_obj.id);

        assert_eq!(recipient_obj.slots.len(), 2);
        assert_eq!(recipient_obj.cap_addr, Some(cap_addr));

        // get the slot one by one
        let slot0: &RecipientSlotObject = recipient_obj.slots.get(0).unwrap();
        println!("slot 0: {:?}", slot0);
        assert_eq!(slot0.slot_id, 0);
        assert_eq!(slot0.token_addr, COIN_SUI_PATH.to_string());
        assert_eq!(slot0.slot_type, RecipientSlotType::Token);
        assert_eq!(slot0.balance, 0);
        assert_eq!(slot0.shares.len(), 2);
        assert_eq!(slot0.shares[0].owner,
                   RecipientSlotOwner::Unassigned { identifier: "Race1".to_string()});
        assert_eq!(slot0.shares[0].weights, 10);
        assert_eq!(slot0.shares[0].claim_amount, 0);

        let slot1: &RecipientSlotObject = recipient_obj.slots.get(1).unwrap();
        assert_eq!(slot1.slot_id, 1);
        assert_eq!(slot1.token_addr, COIN_SUI_PATH.to_string());
        assert_eq!(slot1.slot_type, RecipientSlotType::Nft);
        assert_eq!(slot1.balance, 0);
        assert_eq!(slot1.shares.len(), 2);
        assert_eq!(slot1.shares[1].owner,
                   RecipientSlotOwner::Assigned { addr: parse_sui_addr(PUBLISHER)? });
        Ok(())
    }

    #[tokio::test]
    async fn test_get_recipient_account() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let result = transport.get_recipient(TEST_RECIPIENT_ID).await?;
        assert!(result.is_some());

        let recipient_account = result.unwrap();
        assert_eq!(recipient_account.slots.len(), 2);
        let slot0: race_core::types::RecipientSlot = recipient_account.slots[0].clone();
        assert_eq!(slot0.shares.len(), 2);
        assert_eq!(slot0.id, 0);
        assert_eq!(slot0.token_addr, COIN_SUI_PATH.to_string());
        assert_eq!(slot0.slot_type, RecipientSlotType::Token);
        assert_eq!(slot0.balance, 0);
        assert_eq!(slot0.shares.len(), 2);
        assert_eq!(slot0.shares[0].owner,
                   race_core::types::RecipientSlotOwner::Unassigned {
                       identifier: "Race1".to_string()});
        assert_eq!(slot0.shares[0].weights, 10);
        assert_eq!(slot0.shares[0].claim_amount, 0);

        let slot1: race_core::types::RecipientSlot = recipient_account.slots[1].clone();
        assert_eq!(slot1.id, 1);
        assert_eq!(slot1.token_addr, COIN_SUI_PATH.to_string());
        assert_eq!(slot1.slot_type, race_core::types::RecipientSlotType::Nft);
        assert_eq!(slot1.balance, 0);
        assert_eq!(slot1.shares.len(), 2);
        assert_eq!(slot1.shares[1].owner,
                   race_core::types::RecipientSlotOwner::Assigned {
                       addr: PUBLISHER.to_string() });

        Ok(())
    }

    #[tokio::test]
    async fn test_create_registration() -> Result<()> {
        let params = CreateRegistrationParams {
            is_private: false,
            size: 20
        };
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let object_id = transport.create_registration(params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_registration() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        // create registration
        // let reg_str_id = transport.create_registration( CreateRegistrationParams {
        //     is_private: false,
        //     size: 30
        // }).await?;
        // get the registration
        let result = transport.get_registration(TEST_REGISTRY).await?;
        assert!(result.is_some());

        let reg_account = result.unwrap();
        assert_eq!(reg_account.addr, TEST_REGISTRY);
        assert_eq!(reg_account.is_private, false);
        assert_eq!(reg_account.size, 100);
        assert_eq!(reg_account.games.len(), 0);
        assert!(reg_account.owner.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_register_game() -> Result<()> {
        // create game
        let game_params = make_game_params();
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();

        let game_addr = TEST_CASH_GAME_ID.to_string();
        let reg_params = RegisterGameParams {
            game_addr: game_addr.clone(),
            reg_addr: TEST_REGISTRY.to_string()
        };

        // register the newly created game
        transport.register_game(reg_params).await?;

        // get the registry to check if the game is successfully registered
        let result = transport.get_registration(TEST_REGISTRY).await?;
        assert!(result.is_some());

        let reg_account = result.unwrap();
        assert!(reg_account.games.len() >= 1);
        assert_eq!(
            reg_account.games.iter().find(|g| g.addr == game_addr).map(|g| g.addr.clone()),
            Some(game_addr)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_unregister_game() -> Result<()> {
        // create game
        let game_params = make_game_params();
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let game_addr: String = transport.create_game_account(game_params).await?;

        // register this new game in the test registry (created before hand)
        let reg_params = RegisterGameParams {
            game_addr: game_addr.clone(),
            reg_addr: TEST_REGISTRY.to_string()
        };
        transport.register_game(reg_params).await?;

        // make sure the game is registered
        let result1 = transport.get_registration(TEST_REGISTRY).await?;
        assert!(result1.is_some());

        let reg_account1 = result1.unwrap();
        let game_num1 = reg_account1.games.len();
        assert!(game_num1 >= 1);
        assert_eq!(
            reg_account1.games.iter().find(|g| g.addr == game_addr).map(|g| g.addr.clone()),
            Some(game_addr.clone())
        );

        // now unregister it
        let unreg_params = UnregisterGameParams {
            game_addr: game_addr.to_string(),
            reg_addr: TEST_REGISTRY.to_string()
        };
        transport.unregister_game(unreg_params).await?;

        // check the result
        let result2 = transport.get_registration(TEST_REGISTRY).await?;
        assert!(result2.is_some());

        let reg_account2 = result2.unwrap();
        let game_num2 = reg_account2.games.len();
        assert_eq!(game_num2, game_num1 - 1);
        assert_eq!(
            reg_account2.games.iter().find(|g| g.addr == game_addr).map(|g| g.addr.clone()),
            None
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_join_game() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let join_params = JoinParams {
            game_addr: TEST_CASH_GAME_ID.to_string(),
            access_version: 0,
            amount: 1_500_000_000,
            position: 1,
            verify_key: "player1".to_string()
        };
        transport.join(join_params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_create_game() -> Result<()> {
        let params = make_game_params();
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let game_id = transport.create_game_account(params).await?;
        println!("[Test]: Created game object with id: {}", game_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_game_account() -> Result<()> {
        // create a game and then try to get it from the chain (devnet)
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let params = make_game_params();
        let game_id_str = transport.create_game_account(params).await?;
        let game_id = parse_object_id(&game_id_str)?;

        // test getting game object
        let game_obj = transport.get_move_object::<GameObject>(game_id).await?;
        assert_eq!(game_obj.title, "Race Devnet Test".to_string());
        assert_eq!(game_obj.access_version, 0);
        assert_eq!(game_obj.settle_version, 0);
        assert_eq!(game_obj.max_players, 10);
        assert_eq!(game_obj.transactor_addr, None);
        assert_eq!(game_obj.players, vec![]);
        assert_eq!(game_obj.entry_type, EntryType::Ticket {amount: 100});
        assert_eq!(game_obj.entry_lock, EntryLock::Open);

        // test getting game object and convert it to account
        Ok(())
    }

    #[tokio::test]
    async fn test_register_server() -> Result<()> {
        let params = RegisterServerParams {
            endpoint: "https://race.poker".to_string(),
        };
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        transport.register_server(params).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_server_account() -> Result<()> {
        // create a server
        let params = RegisterServerParams {
            endpoint: "https://race.poker".to_string(),
        };
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        transport.register_server(params).await?;

        // get the server
        let server_ref: ObjectRef = transport.get_owned_object_ref(
            parse_sui_addr(PUBLISHER)?,
            SuiObjectDataFilter::StructType(
                new_structtag(
                    &format!("{}::{}::{}", transport.package_id, "server", "Server"),
                    None
                )?
            )
        ).await?;
        let server_id_string = server_ref.0.to_hex_uncompressed();

        // test
        let result = transport.get_server_account(&server_id_string).await?;
        assert!(result.is_some());
        let server_account = result.unwrap();
        assert_eq!(server_account.addr, server_id_string);
        assert_eq!(server_account.endpoint, "https://race.poker".to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_serve_game() -> Result<()> {
        let params = ServeParams {
            game_addr: TEST_CASH_GAME_ID.to_string(),
            verify_key: "RaceTest1".to_string()
        };
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        transport.serve(params).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_close_game_account() -> Result<()> {
        // create a game for deletion purposes
        let params = make_game_params();
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let game_id: String = transport.create_game_account(params).await?;

        // delete it
        let dparams = CloseGameAccountParams { addr: game_id };
        transport.close_game_account(dparams).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_attach_coin_bonus() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        // attach coin bonus to it
        let bonus_params = AttachBonusParams {
            game_id: parse_object_id(TEST_CASH_GAME_ID)?,
            token_addr: COIN_SUI_PATH.to_string(),
            identifier: "RaceSuiBonus".to_string(),
            amount: 100_000_000, // 0.1 SUI
            bonus_type: BonusType::Coin(COIN_SUI_PATH.to_string()),
            filter: None
        };
        transport.attach_bonus(bonus_params).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_publish_game() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();

        // publish game
        let publish_params = PublishGameParams {
            uri: "https://arweave.net/rb0z--jgbT3-4hBFXGR5esnRPGTj7aSeh_-qc-ucTfk".to_string(),
            name: "RaceSuiTestNFT".to_string(),
            symbol: "RACESUI".to_string()
        };
        let _nft_id = transport.publish_game(publish_params).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_attach_nft_bonus() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        // attach coin bonus to it
        let nft_id = "0x1a5b13088a9a5dcafea2f4ae4996b7b6995bc281ecb600ffd8458ed0d6b78e4c";
        let bonus_params = AttachBonusParams {
            game_id: parse_object_id(TEST_CASH_GAME_ID)?,
            token_addr: COIN_SUI_PATH.to_string(),
            identifier: "RaceSuiNFT".to_string(),
            amount: 0,
            bonus_type: BonusType::Object(parse_object_id(nft_id)?),
            filter: None
        };
        transport.attach_bonus(bonus_params).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_settle_game() -> Result<()> {
        let transport = SuiTransport::try_new(
            SUI_DEVNET_URL.into(),
            TEST_PACKAGE_ID,
            None
        ).await.unwrap();
        let params = SettleParams {
            addr: "".to_string(),
            settles: vec![],
            transfer: None,
            awards: vec![],
            checkpoint: CheckpointOnChain { root: vec![], size: 0, access_version: 0 },
            access_version: 0,
            settle_version: 0,
            next_settle_version: 0,
            entry_lock: Some(EntryLock::Closed),
            accept_deposits: vec![0, 1],
        };
        let result = transport.settle_game(params).await?;
        Ok(())
    }
}

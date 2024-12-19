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
use sui_json_rpc_types::{Coin, CoinPage, ObjectChange, SuiTransactionBlockResponse, SuiTransactionBlockEffectsAPI};
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

use crate::error::{TransportError, TransportResult};
use race_core::{
    error::{Error, Result},
    transport::TransportT,
    types::{
        AssignRecipientParams, CloseGameAccountParams, CreateGameAccountParams,
        CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams, EntryType,
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
        let gas_fees = self.estimate_gas(pt.clone()).await?;
        let balance = self.get_balance(Some(params.token_addr.clone())).await?;
        if balance < gas_fees {
            return Err(Error::TransportError("InsufficientBalance".into()));
        }

        // actually send the tx
        let coin = self.get_coin(Some(params.token_addr)).await?;
        let gas_price = self.get_gas_price().await?;
        let tx_data = TransactionData::new_programmable(
            self.get_active_addr(),
            vec![coin.object_ref()],
            pt,
            balance,
            gas_price
        );

        let response = self.send_transaction(tx_data).await?;

        if let Some(object_changes) = response.object_changes {
            for object in object_changes {
                match object {
                    ObjectChange::Created { object_id,  .. } => {
                        info!("Created game object with id: {}",
                              object_id.to_hex_uncompressed())
                    },
                    _ => {}
                }
            }
        } else {
            return Err(Error::TransportError("No game created".into()));
        }

        // TODO: return the above object id
        Ok(response.digest.to_string())
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
                    .with_events(),
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

    // `String` represents full Coin path such as "0x2::sui::SUI" (if None)
    async fn get_coin(&self, coin_type: Option<String>) -> TransportResult<Coin> {
        let coins: CoinPage = self.client
            .coin_read_api()
            .get_coins(self.get_active_addr(), coin_type, None, Some(50))
            .await?;
        coins.data.into_iter().next().ok_or(TransportError::NoCoinFound)
    }

    async fn get_gas_price(&self) -> TransportResult<u64> {
        Ok(self.client.read_api().get_reference_gas_price().await?)
    }

    // get available balance from coins and check if it is sufficient
    async fn get_balance(&self, coin_type: Option<String>) -> TransportResult<u64> {
        let coin_page: CoinPage = self.client
            .coin_read_api()
            .get_coins(self.get_active_addr(), coin_type, None, Some(50))
            .await?;
        let balance = coin_page.data.into_iter().map(|c: Coin| c.balance).sum();
        Ok(balance)
    }

    async fn estimate_gas(
        &self,
        pt: ProgrammableTransaction
    ) -> TransportResult<u64> {
        let gas_price = self.client.read_api().get_reference_gas_price().await?;
        let coin = self.get_coin(None).await?;
        let tx_data = TransactionData::new_programmable(
            self.get_active_addr(),
            vec![coin.object_ref()],
            pt,
            GAS_BUDGET,
            gas_price
        );
        let dry_run = self.client.read_api()
            .dry_run_transaction_block(tx_data)
            .await?;
        let cost_summary = dry_run.effects.gas_cost_summary();
        let net_gas_fees: i64 = cost_summary.net_gas_usage();

        if net_gas_fees < 0 {
            return Err(TransportError::NegativeGas(net_gas_fees));
        };

        info!("Estimated gas fees: {} in MIST", net_gas_fees);
        // add a small buffer to the estimated gas fees
        Ok(net_gas_fees as u64 + 50)
    }

    async fn send_transaction(
        &self,
        tx_data: TransactionData
    ) -> TransportResult<SuiTransactionBlockResponse> {
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
    async fn test_create_game() -> TransportResult<()> {
        let params = CreateGameAccountParams {
            title: "Race Sui".into(),
            bundle_addr: SuiTransport::rand_account_str_addr(),
            token_addr: COIN_SUI_ADDR.into(),
            max_players: 6,
            entry_type: EntryType::Cash {min_deposit: 10, max_deposit: 100},
            recipient_addr: SuiTransport::rand_account_str_addr(),
            data: vec![8u8, 1u8, 2u8, 3u8, 4u8],
        };

        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), PACKAGE_ID).await?;

        let digest = transport.create_game_account(params).await?;

        println!("Create game object tx digest: {}", digest);

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

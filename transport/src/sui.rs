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
    rpc_types::{SuiMoveStruct, SuiObjectDataFilter, SuiObjectResponse, SuiObjectResponseQuery, SuiParsedData, SuiParsedMoveObject, SuiTransactionBlockResponseOptions}, types::{
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
        RegistrationAccount, ServeParams, ServerAccount, SettleParams, SettleResult, Transfer, UnregisterGameParams, VoteParams,
RecipientSlotInit, RecipientSlotType, RecipientSlotOwner, RecipientSlotShare, RecipientSlotShareInit,
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
    // local key file store
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

    // generate a random pubkey for some testing cases
    pub fn random_keypair() -> SuiAddress {
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

        // coin for gas
        let coins = self.client
            .coin_read_api()
            .get_coins(self.get_active_addr(), None, None, None)
            .await?;
        let coin = coins.data.into_iter().next().unwrap();

        let mut ptb = PTB::new();
        let module = new_identifier(RECIPIENT)?;
        // create hot potato: recipient builder
        let recipient_builder_fun = new_identifier(NEW_RECIPIENT_BUILDER)?;
        ptb.command(Command::move_call(
            self.get_package_id(),
            module.clone(),
            recipient_builder_fun,
            vec![],
            vec![],
        ));

        let recipient_slot_fun = new_identifier(CREATE_RECIPIENT_SLOT)?;
        let slot_type_fun = new_identifier(BUILD_SLOT_TYPE)?;
        // track result index
        let mut result_idx = 0u16;
        for slot in params.slots.into_iter() {
            // build RecipientSlotType enum on chain and use it as return value
            let slot_type = match slot.slot_type {
                RecipientSlotType::Nft => CallArg::Pure(vec![0u8]),
                RecipientSlotType::Token => CallArg::Pure(vec![1u8]),
            };
            ptb.input(slot_type).map_err(|_| Error::ExternalError("Failed to add input slot type".into()))?;
            ptb.command(Command::move_call(
                self.get_package_id(),
                module.clone(),
                slot_type_fun.clone(),
                vec![],
                vec![Argument::Input(0)],
            ));

            // prepare inputs for subsequent movecalls
            ptb.input(new_callarg(&slot.id)?)
                .map_err(|_| Error::ExternalError("Failed to add input slot id".into()))?;
            let addr = parse_str_addr(&slot.token_addr)?;
            ptb.input(new_callarg(&addr)?)
                .map_err(|_| Error::ExternalError("Failed to add input token addr".into()))?;

            // add move call to `create_recipient_slot`
            ptb.command(Command::move_call(
                self.get_package_id(),
                module.clone(),
                recipient_slot_fun.clone(),
                vec![],
                vec![
                    Argument::Input(1), // slot id
                    Argument::Input(2), // token address
                    Argument::Result(result_idx), // builder
                    Argument::Result(result_idx+1), // slot type
                    // TODO: add slot shares here
                ],
            ));

            result_idx += 2;
        }

        let cap_addr = match params.cap_addr {
            Some(addr_str) => Some(parse_str_addr(&addr_str)?),
            None => None,
        };

        let cap_addr_arg = ptb.pure(cap_addr)
            .map_err(|e| Error::InternalError(format!("Failed to create cap_addr argument: {}", e)))?;

        let recipient_fun = new_identifier(CREATE_RECIPIENT)?;

        ptb.command(Command::move_call(
            self.get_package_id(),
            module.clone(),
            recipient_fun,
            vec![],  // no type arguments
            vec![cap_addr_arg, Argument::Result(result_idx)],
        ));
        let gas_price = self.client.read_api().get_reference_gas_price().await?;

         // build and execute the transaction
        let tx_data = TransactionData::new_programmable(
            self.get_active_addr(),
            vec![coin.object_ref()],
            ptb.finish(),
            GAS_BUDGET,
            gas_price,
        );

        // sign and execute transaction
        let signature = self.keystore.sign_secure(
            &self.active_addr,
            &tx_data,
            Intent::sui_transaction(),
        )
            .map_err(|_| Error::ExternalError("Failed to sign tx".into()))?;

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
        let package_id = ObjectID::from_hex_literal(PACKAGE_ID).unwrap();
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), package_id).await.unwrap();
        let profile = transport.get_player_profile("0x13c43bafded256bdfda2e0fe086785aefa6e4ff45fb14fc3ca747b613aa12902").await;
    }

    #[tokio::test]
    async fn test_create_sui_transport() ->  TransportResult<()> {
        let package_id = ObjectID::from_hex_literal(PACKAGE_ID)
            .map_err(|_| TransportError::ParseObjectIdError(PACKAGE_ID.into()))?;
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), package_id).await?;
        let address = transport.keystore.clone();
        println!("Prepare to get balances");
        let total_balance = transport.client
            .coin_read_api()
            .get_all_balances(address)
            .await?;
        println!("The balances for all coins owned by address: {address} are {:?}", total_balance);
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
                    token_addr: PUBLISHER.into(),
                    init_shares: vec![
                        RecipientSlotShareInit {
                            owner: RecipientSlotOwner::Unassigned {
                                identifier: "Race".into()
                            },
                            weights: 10,
                        },
                        RecipientSlotShareInit {
                            owner: RecipientSlotOwner::Unassigned {
                                identifier: "Race".into()
                            },
                            weights: 20,
                        }
                    ],
                }
            ]
        };
        let package_id = ObjectID::from_hex_literal(PACKAGE_ID)
            .map_err(|_| TransportError::ParseObjectIdError(PACKAGE_ID.into()))?;
        let transport = SuiTransport::try_new(SUI_DEVNET_URL.into(), package_id).await?;

        let res = transport.create_recipient(params).await?;
        println!("Create recipient tx digest: {}", res);

        Ok(())
    }
}

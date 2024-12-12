#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
/// Transport for Sui blockchain
mod constants;
mod types;

use async_trait::async_trait;
use constants::*;
use futures::{Stream, StreamExt};
use bcs;
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use shared_crypto::intent::Intent;
use sui_config::{sui_config_dir, SUI_KEYSTORE_FILENAME};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_sdk::{
    rpc_types::{SuiMoveStruct, SuiObjectDataFilter, SuiObjectResponse, SuiObjectResponseQuery, SuiParsedData, SuiParsedMoveObject, SuiTransactionBlockResponseOptions}, types::{
        base_types::{ObjectID, SuiAddress}, crypto::{get_key_pair_from_rng, SuiKeyPair}, programmable_transaction_builder::ProgrammableTransactionBuilder as PTB, quorum_driver_types::ExecuteTransactionRequestType, sui_serde::HexAccountAddress, transaction::{Argument, CallArg, Command, Transaction, TransactionData}, Identifier
    }, SuiClient, SuiClientBuilder, SUI_COIN_TYPE, SUI_DEVNET_URL
};
use tracing::{error, info, warn};
use types::*;
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
        RegistrationAccount, ServeParams, ServerAccount, SettleParams, SettleResult, Transfer,
        UnregisterGameParams, VoteParams,
        // RecipientSlotInit, RecipientSlotType, RecipientSlotOwner, RecipientSlotShare, RecipientSlotShareInit,
    }
};

pub struct SuiTransport {
    rpc: String,
    package_id: ObjectID,
    keypair: SuiAddress,
    client: SuiClient,
}

// pub fn retrieve_wallet() -> TransportResult<WalletContext> {
//     let wallet_conf = sui_config_dir()?.join(SUI_CLIENT_CONFIG);
//     let keystore_path = sui_config_dir()?.join(SUI_KEYSTORE_FILENAME);
//
//     // check if a wallet exists and if not, create a wallet and a sui client config
//     if !keystore_path.exists() {
//         let keystore = FileBasedKeystore::new(&keystore_path)?;
//         keystore.save()?;
//     }
//
//     if !wallet_conf.exists() {
//         let keystore = FileBasedKeystore::new(&keystore_path)?;
//         let mut client_config = SuiClientConfig::new(keystore.into());
//
//         client_config.add_env(SuiEnv::testnet());
//         client_config.add_env(SuiEnv::devnet());
//         client_config.add_env(SuiEnv::localnet());
//
//         if client_config.active_env.is_none() {
//             client_config.active_env = client_config.envs.first().map(|env| env.alias.clone());
//         }
//
//         client_config.save(&wallet_conf)?;
//         info!("Client config file is stored in {:?}.", &wallet_conf);
//     }
//
//     let mut keystore = FileBasedKeystore::new(&keystore_path)?;
//     let mut client_config: SuiClientConfig = PersistedConfig::read(&wallet_conf)?;
//
//     let default_active_address = if let Some(address) = keystore.addresses().first() {
//         *address
//     } else {
//         keystore
//             .generate_and_add_new_key(ED25519, None, None, None)?
//             .0
//     };
//
//     if keystore.addresses().len() < 2 {
//         keystore.generate_and_add_new_key(ED25519, None, None, None)?;
//     }
//
//     client_config.active_address = Some(default_active_address);
//     client_config.save(&wallet_conf)?;
//
//     let wallet = WalletContext::new(&wallet_conf, Some(std::time::Duration::from_secs(60)), None)?;
//
//     Ok(wallet)
// }

impl SuiTransport {
    pub(crate) async fn try_new(rpc: String, package_id: ObjectID) -> TransportResult<Self> {
        println!(
            "Create Sui transport at RPC: {} for packge id: {:?}",
            rpc, package_id
        );
        let client = SuiClientBuilder::default().build(rpc.clone()).await?;
        let keypair = SuiAddress::from_str(SUI_ACCOUNT)
            .map_err(|_| TransportError::ParseAddressError)?;
        Ok(Self {
            rpc,
            package_id,
            keypair,
            client
        })
    }

    // generate a random pubkey for some testing cases
    pub fn random_pubkey() -> SuiAddress {
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

        let module_name = "recipient";
        let builder_func_name = "new_recipient_builder";
        let recipient_func_name = "create_recipient";
        let recipient_slot_func_name = "create_recipient_slot";
        let active_addr = SuiAddress::from_str("0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192").map_err(|_| TransportError::ParseAddressError)?;
        // coin for gas
        let coins = self.client
            .coin_read_api()
            .get_coins(active_addr.clone(), None, None, None)
            .await?;
        let coin = coins.data.into_iter().next().unwrap();

        let mut ptb = PTB::new();
        let module = Identifier::new(module_name)
            .map_err(|_| TransportError::FailedToIdentifySuiModule(module_name.into()))?;
        let builder_func = Identifier::new(builder_func_name)
            .map_err(|_| TransportError::FailedToIdentifySuiModuleFn(builder_func_name.into()))?;
        ptb.command(Command::move_call(
            self.package_id.clone(),
            module.clone(),
            builder_func,
            vec![],
            vec![],
        ));
        let keystore = FileBasedKeystore::new(&sui_config_dir()?.join(SUI_KEYSTORE_FILENAME))?;

        let recipient_func = Identifier::new(recipient_func_name)
            .map_err(|_| TransportError::FailedToIdentifySuiModuleFn(recipient_func_name.into()))?;

        let cap_addr = match params.cap_addr {
            Some(addr_str) => Some(SuiAddress::from_str(&addr_str)
                    .map_err(|e| Error::ExternalError(format!("Invalid cap address: {}", e)))?),
            None => None,
        };

         let cap_addr_arg = ptb.pure(cap_addr)
            .map_err(|e| Error::InternalError(format!("Failed to create cap_addr argument: {}", e)))?;

        let recipient_func = Identifier::new(recipient_func_name)
            .map_err(|_| Error::InternalError("Failed to create recipient function identifier".into()))?;

        ptb.command(Command::move_call(
            self.package_id.clone(),
            module.clone(),
            recipient_func,
            vec![],  // no type arguments
            vec![
                cap_addr_arg,
                Argument::Result(0),
            ],
        ));
        let gas_price = self.client.read_api().get_reference_gas_price().await?;

         // build and execute the transaction
        let tx_data = TransactionData::new_programmable(
            active_addr.clone(),
            vec![coin.object_ref()],
            ptb.finish(),
            GAS_BUDGET,
            gas_price,
        );

        // sign and execute transaction
        let signature = keystore.sign_secure(
            &active_addr,
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
        let address = transport.keypair.clone();
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
                    token_addr: "0x02::Sui".into(),
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

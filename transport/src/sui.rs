#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
/// Transport for Sui blockchain
mod constants;

use async_trait::async_trait;
use constants::*;
use futures::{Stream, StreamExt};
use sui_sdk::{
    types::{
        base_types::{ObjectID, SuiAddress},
        crypto::{get_key_pair_from_rng, SuiKeyPair},
    },
    SuiClient, SuiClientBuilder,
    SUI_DEVNET_URL, SUI_COIN_TYPE,

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
        RegistrationAccount, ServeParams, ServerAccount, SettleParams, SettleResult, Transfer,
        UnregisterGameParams, VoteParams,
    }
};

pub struct SuiTransport {
    rpc: String,
    package_id: ObjectID,
    keypair: SuiAddress,
    client: SuiClient,
}

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

    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        todo!()
    }

    async fn create_recipient(&self, params: CreateRecipientParams) -> Result<String> {
        todo!()
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
        todo!()
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
    use sui_sdk::types::base_types::{ ObjectID, SuiAddress };

    #[tokio::test]
    async fn test_create_sui_transport() ->  TransportResult<()> {
        let package_id = ObjectID::from_str(PACKAGE_ID)
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
}

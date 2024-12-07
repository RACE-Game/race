/// Transport for Sui blockchain

use async_trait::async_trait;
use sui_sdk::{
    types::base_types::{SuiAddress},
    SuiClient, SuiClientBuiler
};
use tracing::{error, info, warn};

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
    package_id: SuiAddress,
    client: SuiClient,
}

#[async_trait]
impl TransportT for SuiTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        Ok("")
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        Ok("")
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        Ok("")
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        Ok(())
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        Ok(())
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        Ok(())
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        Ok(())
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        Ok(({}))
    }

    async fn create_recipient(&self, params: CreateRecipientParams) -> Result<String> {
        Ok("")
    }

    async fn recipient_claim(&self, params: RecipientClaimParams) -> Result<()> {
        Ok(())
    }

    async fn assign_recipient(&self, params: AssignRecipientParams) -> Result<()> {
        Ok(())
    }

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        Ok("")
    }

    async fn settle_game(&self, params: SettleParams) -> Result<SettleResult> {
        Ok(SettleResult {

        })
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        Ok("")
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn get_game_account(&self, addr: &str) -> Result<Option<GameAccount>> {
        Ok(Some(GameAccount {

        }))
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

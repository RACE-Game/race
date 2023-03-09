use async_trait::async_trait;
use race_core::error::Result;

use race_core::types::{
    CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, RegisterGameParams,
    RegisterServerParams, RegistrationAccount, ServeParams, ServerAccount, UnregisterGameParams,
    VoteParams,
};
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, SettleParams,
    },
};


// use solana_sdk::commitment_config::CommitmentConfig;
// use solana_sdk::pubkey::Pubkey;

pub struct SolanaTransport {}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for SolanaTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        Ok("".into())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        todo!()
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        todo!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        todo!()
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        todo!()
    }
    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        todo!()
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        todo!()
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        todo!()
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        todo!()
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<String> {
        Ok("".into())
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        Ok("".into())
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        None
    }
}

impl SolanaTransport {
    pub fn new(_rpc: String) -> Self {
        Self {}
    }
}

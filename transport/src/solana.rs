use std::str::FromStr;

use async_trait::async_trait;
use race_core::error::Result;

use race_core::types::{
    CreateRegistrationParams, GetRegistrationParams, RegisterGameParams, RegisterTransactorParams, RegistrationAccount,
    TransactorAccount, UnregisterGameParams,
};
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, JoinParams, PlayerProfile,
        SettleParams,
    },
};

use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;

pub struct SolanaTransport {
    client: RpcClient,
}

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

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let pubkey = Pubkey::from_str(addr).unwrap();
        let data = self.client.get_account_data(&pubkey).unwrap();
        Some(GameAccount {
            addr: addr.to_owned(),
            bundle_addr: "".into(),
            served: true,
            settle_version: 0,
            access_version: 0,
            max_players: 2,
            transactors: vec![],
            players: vec![],
            data_len: data.len() as _,
            data,
        })
    }
    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        todo!()
    }

    async fn get_transactor_account(&self, addr: &str) -> Option<TransactorAccount> {
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

    async fn register_transactor(&self, params: RegisterTransactorParams) -> Result<()> {
        Ok(())
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

    async fn get_registration(&self, params: GetRegistrationParams) -> Option<RegistrationAccount> {
        None
    }
}

impl SolanaTransport {
    pub fn new(rpc: &str) -> Self {
        let client = RpcClient::new_with_commitment(rpc, CommitmentConfig::finalized());
        Self { client }
    }
}

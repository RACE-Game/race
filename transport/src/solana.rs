use std::str::FromStr;

use async_trait::async_trait;
use race_core::error::{Error, Result};

use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, JoinParams, PlayerProfile,
        SettleParams,
    },
};

use solana_client::rpc_client::{RpcClient, RpcClientConfig};
use solana_sdk::commitment_config::{CommitmentLevel, CommitmentConfig};
use solana_sdk::pubkey::Pubkey;

pub struct SolanaTransport {
    client: RpcClient,
}

#[async_trait]
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
            settle_serial: 0,
            access_serial: 0,
            players: vec![],
            data_len: data.len() as _,
            data,
        })
    }
    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
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
}

impl SolanaTransport {
    pub fn new(rpc: &str) -> Self {
        let client = RpcClient::new_with_commitment(rpc, CommitmentConfig::finalized());
        Self { client }
    }
}

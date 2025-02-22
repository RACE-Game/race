use race_core::{
    error::Result,
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, PublishParams, RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams,
        ServerAccount, SettleParams, UnregisterGameParams, VoteParams,
    },
};

use async_trait::async_trait;

pub struct EvmTransport {
    // pub web3: Web3<Http>,
}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for EvmTransport {
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

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<String> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        todo!()
    }

    async fn publish_game(&self, params: PublishParams) -> Result<String> {
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

// impl EvmTransport {
//     pub fn new(rpc: &str) -> Self {
//         let transport = Http::new(rpc).unwrap();
//         let web3 = Web3::new(transport);
//         Self { web3 }
//     }
// }

impl EvmTransport {
    pub fn new(_rpc: String) -> Self {
        Self {}
    }
}

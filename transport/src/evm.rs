// use web3::{transports::Http, Web3};

use race_core::{
    error::Result,
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreateRegistrationParams, GameAccount,
        GameBundle, GetRegistrationParams, JoinParams, PlayerProfile, RegisterGameParams,
        RegisterTransactorParams, RegistrationAccount, SettleParams, TransactorAccount,
        UnregisterGameParams,
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

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        todo!()
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

// impl EvmTransport {
//     pub fn new(rpc: &str) -> Self {
//         let transport = Http::new(rpc).unwrap();
//         let web3 = Web3::new(transport);
//         Self { web3 }
//     }
// }

impl EvmTransport {
    pub fn new(rpc: &str) -> Self {
        Self {}
    }
}

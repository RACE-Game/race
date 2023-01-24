use std::{
    io::Read,
    ops::Deref,
    sync::{Arc, Mutex},
};

use crate::account_helpers::*;
use async_trait::async_trait;
use race_core::{
    error::{Result, Error},
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreateRegistrationParams,
        GameAccount, GameBundle, JoinParams, PlayerProfile,
        RegisterGameParams, RegisterServerParams, Settle, SettleParams,
        ServerAccount, UnregisterGameParams, RegistrationAccount, ServeParams, CreatePlayerProfileParams, DepositParams,
    },
};

pub struct DummyTransport {
    settles: Arc<Mutex<Vec<Settle>>>,
    states: Arc<Mutex<Vec<GameAccount>>>,
}

impl DummyTransport {
    #[allow(dead_code)]
    pub fn get_settles(&self) -> impl Deref<Target = Vec<Settle>> + '_ {
        self.settles.lock().unwrap()
    }

    #[allow(dead_code)]
    pub fn simulate_states(&self, mut states: Vec<GameAccount>) {
        self.states.lock().unwrap().append(&mut states);
    }
}

impl Default for DummyTransport {
    fn default() -> Self {
        Self {
            settles: Arc::new(Mutex::new(vec![])),
            states: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for DummyTransport {
    async fn create_game_account(&self, _params: CreateGameAccountParams) -> Result<String> {
        Ok(game_account_addr())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        Ok(())
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        Ok(())
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        Ok(())
    }

    async fn get_game_account(&self, _addr: &str) -> Option<GameAccount> {
        let mut states = self.states.lock().unwrap();
        if states.is_empty() {
            None
        } else {
            let game_account = states.remove(0);
            Some(game_account)
        }
    }

    async fn get_game_bundle(&self, addr_q: &str) -> Option<GameBundle> {
        let addr = game_bundle_addr();
        if addr.eq(addr_q) {
            let mut f = std::fs::File::open(
                "../target/wasm32-unknown-unknown/release/race_example_counter.wasm",
            )
            .unwrap();
            let mut data = vec![];
            f.read_to_end(&mut data).unwrap();
            Some(GameBundle { addr, data })
        } else {
            None
        }
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        None
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        Ok(bundle.addr)
    }

    async fn settle_game(&self, mut params: SettleParams) -> Result<()> {
        if params.addr.eq(&game_account_addr()) {
            let mut settles = self.settles.lock().unwrap();
            settles.append(&mut params.settles);
            Ok(())
        } else {
            Err(Error::GameAccountNotFound)
        }
    }

    async fn deposit(&self, params: DepositParams) -> Result<()>{
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        Ok(())
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

#[cfg(test)]
mod tests {

    use race_core::types::Settle;

    use super::*;

    #[tokio::test]
    async fn test_get_bundle() {
        let transport = DummyTransport::default();
        let addr = game_bundle_addr();
        let bundle = transport.get_game_bundle(&addr).await.unwrap();
        assert_ne!(0, bundle.data.len());
    }

    #[tokio::test]
    async fn test_get_state() {
        let transport = DummyTransport::default();
        let ga_0 = GameAccount {
            addr: game_account_addr(),
            bundle_addr: game_bundle_addr(),
            access_version: 0,
            ..Default::default()
        };
        let ga_1 = TestGameAccountBuilder::default().add_players(1).build();
        let ga_2 = TestGameAccountBuilder::default().add_players(2).build();
        let states = vec![ga_0.clone(), ga_1.clone(), ga_2.clone()];
        transport.simulate_states(states);

        let addr = game_account_addr();
        assert_eq!(Some(ga_0), transport.get_game_account(&addr).await);
        assert_eq!(Some(ga_1), transport.get_game_account(&addr).await);
        assert_eq!(Some(ga_2), transport.get_game_account(&addr).await);
        assert_eq!(None, transport.get_game_account(&addr).await);
    }

    #[tokio::test]
    async fn test_settle() {
        let transport = DummyTransport::default();
        let settles = vec![
            Settle::add("Alice", 100),
            Settle::add("Bob", 100),
        ];
        let params = SettleParams {
            addr: game_account_addr(),
            settles: settles.clone(),
        };
        transport.settle_game(params.clone()).await.unwrap();
        transport.settle_game(params.clone()).await.unwrap();

        let mut expected = vec![];
        expected.append(&mut settles.clone());
        expected.append(&mut settles.clone());

        assert_eq!(*transport.get_settles(), expected);
    }
}

//! The mock for transactor for testing
use std::io::Read;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use race_core::error::{Error, Result};
use race_core::types::SettleParams;
use race_core::{
    transport::TransportT,
    types::{CreateGameAccountParams, GameAccount, GameBundle, Settle},
};

pub struct MockTransport {
    settles: Arc<Mutex<Vec<Settle>>>,
    states: Arc<Mutex<Vec<GameAccount>>>,
}

impl MockTransport {
    pub fn mock_game_account_addr() -> String {
        "FAKE GAME ACCOUNT ADDR".into()
    }

    pub fn mock_game_bundle_addr() -> String {
        "FAKE GAME BUNDLE ADDR".into()
    }

    #[allow(dead_code)]
    pub fn get_settles<'a>(&'a self) -> impl Deref<Target = Vec<Settle>> + 'a {
        self.settles.lock().unwrap()
    }

    #[allow(dead_code)]
    pub fn simulate_states(&self, mut states: Vec<GameAccount>) {
        self.states.lock().unwrap().append(&mut states);
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self {
            settles: Arc::new(Mutex::new(vec![])),
            states: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl TransportT for MockTransport {
    async fn create_game_account(&self, _params: CreateGameAccountParams) -> Result<String> {
        Ok(MockTransport::mock_game_account_addr())
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
        let addr = MockTransport::mock_game_bundle_addr();
        if addr.eq(addr_q) {
            let mut f = std::fs::File::open("../target/wasm32-unknown-unknown/release/minimal.wasm").unwrap();
            let mut data = vec![];
            f.read_to_end(&mut data).unwrap();
            Some(GameBundle { addr, data })
        } else {
            None
        }
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        Ok(bundle.addr)
    }

    async fn settle_game(&self, mut params: SettleParams) -> Result<()> {
        if params.addr.eq(&MockTransport::mock_game_account_addr()) {
            let mut settles = self.settles.lock().unwrap();
            settles.append(&mut params.settles);
            Ok(())
        } else {
            Err(Error::GameAccountNotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use race_core::{
        transport::TransportT,
        types::{AssetChange, PlayerStatus, Settle, SettleParams, GameAccount, Player},
    };

    use crate::MockTransport;

    #[tokio::test]
    async fn test_get_state() {
        let transport = MockTransport::default();
        let ga_0 = GameAccount {
            addr: MockTransport::mock_game_account_addr(),
            game_addr: MockTransport::mock_game_bundle_addr(),
            access_serial: 0,
            ..Default::default()
        };
        let ga_1 = GameAccount {
            addr: MockTransport::mock_game_account_addr(),
            game_addr: MockTransport::mock_game_bundle_addr(),
            access_serial: 1,
            players: vec![Some(Player::new("Alice", 100))],
            ..Default::default()
        };
        let ga_2 = GameAccount {
            addr: MockTransport::mock_game_account_addr(),
            game_addr: MockTransport::mock_game_bundle_addr(),
            access_serial: 2,
            players: vec![Some(Player::new("Alice", 100)), Some(Player::new("Bob", 200))],
            ..Default::default()
        };
        let states = vec![ga_0.clone(), ga_1.clone(), ga_2.clone()];
        transport.simulate_states(states);

        let addr = MockTransport::mock_game_account_addr();
        assert_eq!(Some(ga_0), transport.get_game_account(&addr).await);
        assert_eq!(Some(ga_1), transport.get_game_account(&addr).await);
        assert_eq!(Some(ga_2), transport.get_game_account(&addr).await);
        assert_eq!(None, transport.get_game_account(&addr).await);
    }

    #[tokio::test]
    async fn test_settle() {
        let transport = MockTransport::default();
        let settles = vec![
            Settle::new("Alice", PlayerStatus::Normal, AssetChange::Add, 100),
            Settle::new("Bob", PlayerStatus::Normal, AssetChange::Add, 100),
        ];
        let params = SettleParams {
            addr: MockTransport::mock_game_account_addr(),
            settles: settles.clone(),
        };
        transport.settle_game(params.clone()).await.unwrap();
        transport.settle_game(params.clone()).await.unwrap();

        let mut expected = vec![];
        expected.append(&mut settles.clone());
        expected.append(&mut settles.clone());

        assert_eq!(
            *transport.get_settles(),
            expected
        );
    }
}

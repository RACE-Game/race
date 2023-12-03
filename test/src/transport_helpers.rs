use std::{
    io::Read,
    ops::Deref,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use base64::prelude::Engine;
use race_core::types::{CreateRecipientParams, AssignRecipientParams, RecipientAccount, RecipientClaimParams};
use race_api::error::{Error, Result};
#[allow(unused_imports)]
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, PublishGameParams, QueryMode, RegisterGameParams, RegisterServerParams,
        RegistrationAccount, ServeParams, ServerAccount, Settle, SettleParams,
        UnregisterGameParams, VoteParams,
    },
};

pub struct DummyTransport {
    settles: Arc<Mutex<Vec<Settle>>>,
    states: Arc<Mutex<Vec<GameAccount>>>,
    fail_next_settle: Arc<Mutex<bool>>,
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

    #[allow(dead_code)]
    pub fn fail_next_settle(&mut self) {
        let mut fail_next_settle = self.fail_next_settle.lock().unwrap();
        *fail_next_settle = true;
    }

    #[allow(dead_code)]
    pub fn default_game_addr() -> String {
        "TEST".into()
    }
}

impl Default for DummyTransport {
    fn default() -> Self {
        Self {
            settles: Arc::new(Mutex::new(vec![])),
            states: Arc::new(Mutex::new(vec![])),
            fail_next_settle: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for DummyTransport {
    async fn create_game_account(&self, _params: CreateGameAccountParams) -> Result<String> {
        Ok(Self::default_game_addr())
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

    async fn vote(&self, params: VoteParams) -> Result<()> {
        Ok(())
    }

    async fn get_game_account(&self, _addr: &str, mode: QueryMode) -> Result<Option<GameAccount>> {
        let mut states = self.states.lock().unwrap();
        if states.is_empty() {
            Ok(None)
        } else {
            let game_account = states.remove(0);
            Ok(Some(game_account))
        }
    }

    async fn get_game_bundle(&self, addr_q: &str) -> Result<Option<GameBundle>> {
        let addr: String = "TEST".into();
        if addr.eq(addr_q) {
            let mut f = std::fs::File::open("../examples/minimal/minimal.wasm").unwrap();
            let mut buf = vec![];
            f.read_to_end(&mut buf).unwrap();
            let base64 = base64::prelude::BASE64_STANDARD;
            let data = base64.encode(buf);
            // FIXME: complete fields
            Ok(Some(GameBundle {
                uri: "".into(),
                name: "".into(),
                data: vec![],
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_server_account(&self, addr: &str) -> Result<Option<ServerAccount>> {
        Ok(None)
    }

    async fn get_player_profile(&self, addr: &str) -> Result<Option<PlayerProfile>> {
        Ok(None)
    }

    async fn publish_game(&self, bundle: PublishGameParams) -> Result<String> {
        Ok("".into())
    }

    async fn settle_game(&self, mut params: SettleParams) -> Result<()> {
        let mut fail_next_settle = self.fail_next_settle.lock().unwrap();
        if *fail_next_settle {
            *fail_next_settle = false;
            Err(Error::TransportError("Mock failure".into()))
        } else if params.addr.eq("TEST") {
            let mut settles = self.settles.lock().unwrap();
            settles.append(&mut params.settles);
            Ok(())
        } else {
            Err(Error::GameAccountNotFound)
        }
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        Ok(())
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
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

    async fn get_registration(&self, addr: &str) -> Result<Option<RegistrationAccount>> {
        Ok(None)
    }

    async fn create_recipient(&self, params: CreateRecipientParams) -> Result<String> {
        Ok("".into())
    }

    async fn assign_recipient(&self, params: AssignRecipientParams) -> Result<()> {
        Ok(())
    }

    async fn get_recipient(&self, addr: &str) -> Result<Option<RecipientAccount>> {
        Ok(None)
    }

    async fn recipient_claim(&self, params: RecipientClaimParams) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use race_core::types::Settle;

    use crate::prelude::{test_game_addr, TestClient, TestGameAccountBuilder};

    use super::*;

    #[tokio::test]
    async fn test_get_bundle() -> anyhow::Result<()> {
        let transport = DummyTransport::default();
        let addr = "TEST".to_string();
        let bundle = transport.get_game_bundle(&addr).await?.unwrap();
        assert_eq!(0, bundle.data.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_state() -> anyhow::Result<()> {
        let transport = DummyTransport::default();
        let alice = TestClient::player("alice");
        let bob = TestClient::player("bob");

        let ga_0 = TestGameAccountBuilder::default().build();
        let ga_1 = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .build();
        let ga_2 = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .add_player(&bob, 100)
            .build();

        let states = vec![ga_0.clone(), ga_1.clone(), ga_2.clone()];
        transport.simulate_states(states);

        let addr = test_game_addr();
        assert_eq!(Some(ga_0), transport.get_game_account(&addr, QueryMode::Finalized).await?);
        assert_eq!(Some(ga_1), transport.get_game_account(&addr, QueryMode::Finalized).await?);
        assert_eq!(Some(ga_2), transport.get_game_account(&addr, QueryMode::Finalized).await?);
        assert_eq!(None, transport.get_game_account(&addr, QueryMode::Finalized).await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_settle() {
        let transport = DummyTransport::default();
        let settles = vec![Settle::add("Alice", 100), Settle::add("Bob", 100)];
        let params = SettleParams {
            addr: test_game_addr(),
            settles: settles.clone(),
            transfers: vec![],
            checkpoint: vec![],
            settle_version: 0,
            next_settle_version: 1,
        };
        transport.settle_game(params.clone()).await.unwrap();
        transport.settle_game(params.clone()).await.unwrap();

        let mut expected = vec![];
        expected.append(&mut settles.clone());
        expected.append(&mut settles.clone());

        assert_eq!(*transport.get_settles(), expected);
    }

    #[tokio::test]
    async fn test_fail_settle() {
        let mut transport = DummyTransport::default();
        transport.fail_next_settle();
        let params = SettleParams {
            addr: test_game_addr(),
            settles: vec![],
            transfers: vec![],
            checkpoint: vec![],
            settle_version: 0,
            next_settle_version: 1,
        };
        assert_eq!(transport.settle_game(params).await.is_err(), true);
    }
}

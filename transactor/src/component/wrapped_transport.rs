//! Wrapped transport, which support retry

use jsonrpsee::core::async_trait;
use race_api::error::Result;
use race_core::types::{
    AssignRecipientParams, CreatePlayerProfileParams, CreateRecipientParams,
    CreateRegistrationParams, DepositParams, PublishGameParams, QueryMode, RecipientAccount,
    RecipientClaimParams, RegisterGameParams, ServeParams, UnregisterGameParams, VoteParams,
};
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, RegisterServerParams, RegistrationAccount, ServerAccount, SettleParams,
    },
};
use race_env::Config;
use race_transport::TransportBuilder;
use std::time::Duration;
use tracing::error;

const RETRY_INTERVAL: u64 = 10;

pub struct WrappedTransport {
    pub(crate) inner: Box<dyn TransportT>,
}

impl WrappedTransport {
    pub async fn try_new(config: &Config) -> Result<Self> {
        let chain: &str = &config
            .transactor
            .as_ref()
            .expect("Missing transactor configuration")
            .chain;
        let transport = TransportBuilder::default()
            .try_with_chain(chain)?
            .try_with_config(config)?
            .build()
            .await?;
        Ok(Self { inner: transport })
    }
}

#[async_trait]
impl TransportT for WrappedTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        self.inner.create_game_account(params).await
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        self.inner.create_player_profile(params).await
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        self.inner.close_game_account(params).await
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        self.inner.join(params).await
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        self.inner.serve(params).await
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        self.inner.vote(params).await
    }

    async fn get_game_account(&self, addr: &str, mode: QueryMode) -> Result<Option<GameAccount>> {
        self.inner.get_game_account(addr, mode).await
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        self.inner.deposit(params).await
    }

    async fn get_game_bundle(&self, addr: &str) -> Result<Option<GameBundle>> {
        self.inner.get_game_bundle(addr).await
    }

    async fn get_server_account(&self, addr: &str) -> Result<Option<ServerAccount>> {
        self.inner.get_server_account(addr).await
    }

    async fn get_player_profile(&self, addr: &str) -> Result<Option<PlayerProfile>> {
        self.inner.get_player_profile(addr).await
    }

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        self.inner.publish_game(params).await
    }

    /// `settle_version` is used to identify the settle state,
    /// Until the `settle_version` is bumped, we keep retrying.
    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        let mut curr_settle_version: Option<u64> = None;
        loop {
            let game_account = self
                .inner
                .get_game_account(&params.addr, QueryMode::Finalized)
                .await;
            if let Ok(Some(game_account)) = game_account {
                // We got an old state, which has a smaller `settle_version`
                if game_account.settle_version < params.settle_version {
                    error!(
                        "Got invalid settle_version: {} != {}, will retry in {} secs",
                        game_account.settle_version, params.settle_version, RETRY_INTERVAL
                    );
                    tokio::time::sleep(Duration::from_secs(RETRY_INTERVAL)).await;
                    continue;
                }
                // The `settle_version` had been bumped, indicates the transaction was succeed
                // NOTE: The transaction can success with error result due to unstable network
                if curr_settle_version.is_some_and(|v| v < game_account.settle_version) {
                    return Ok(());
                }
                curr_settle_version = Some(game_account.settle_version);
                if let Err(e) = self.inner.settle_game(params.clone()).await {
                    error!(
                        "Error in settlement: {:?}, will retry in {} secs",
                        e, RETRY_INTERVAL
                    );
                    tokio::time::sleep(Duration::from_secs(RETRY_INTERVAL)).await;
                    continue;
                } else {
                    return Ok(());
                }
            } else {
                error!(
                    "Error in settlement due to unable to get game account {}, will retry in {} secs",
                    params.addr,
                    RETRY_INTERVAL
                );
                tokio::time::sleep(Duration::from_secs(RETRY_INTERVAL)).await;
                continue;
            }
        }
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        self.inner.register_server(params).await
    }

    async fn get_registration(&self, addr: &str) -> Result<Option<RegistrationAccount>> {
        self.inner.get_registration(addr).await
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        self.inner.create_registration(params).await
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        self.inner.register_game(params).await
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        self.inner.unregister_game(params).await
    }

    async fn create_recipient(&self, params: CreateRecipientParams) -> Result<String> {
        self.inner.create_recipient(params).await
    }

    async fn assign_recipient(&self, params: AssignRecipientParams) -> Result<()> {
        self.inner.assign_recipient(params).await
    }

    async fn get_recipient(&self, addr: &str) -> Result<Option<RecipientAccount>> {
        self.inner.get_recipient(addr).await
    }

    async fn recipient_claim(&self, params: RecipientClaimParams) -> Result<()> {
        self.inner.recipient_claim(params).await
    }
}

#[cfg(test)]
mod tests {
    use race_test::prelude::{test_game_addr, DummyTransport, TestGameAccountBuilder};

    use super::*;

    #[tokio::test]
    async fn test_settle_without_retry() -> anyhow::Result<()> {
        let t = DummyTransport::default();
        let ga0 = TestGameAccountBuilder::new().build();
        let mut ga1 = TestGameAccountBuilder::new().build();
        ga1.settle_version = 1;
        t.simulate_states(vec![ga0, ga1]);
        let wt = WrappedTransport { inner: Box::new(t) };
        let r = wt
            .settle_game(SettleParams {
                addr: test_game_addr(),
                settles: vec![],
                transfers: vec![],
                checkpoint: vec![],
                settle_version: 1,
                next_settle_version: 2,
            })
            .await;

        assert_eq!(r, Ok(()));
        Ok(())
    }

    #[tokio::test]
    async fn test_settle_with_retry() -> anyhow::Result<()> {
        let mut t = DummyTransport::default();
        t.fail_next_settle();
        let ga0 = TestGameAccountBuilder::new().build();
        let mut ga1 = TestGameAccountBuilder::new().build();
        ga1.settle_version = 1;
        t.simulate_states(vec![ga0, ga1]);
        let wt = WrappedTransport { inner: Box::new(t) };
        let r = wt
            .settle_game(SettleParams {
                addr: test_game_addr(),
                settles: vec![],
                transfers: vec![],
                checkpoint: vec![],
                settle_version: 1,
                next_settle_version: 2,
            })
            .await;

        assert_eq!(r, Ok(()));
        Ok(())
    }
}

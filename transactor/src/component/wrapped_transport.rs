//! Wrapped transport, which support retry

use async_stream::stream;
use futures::Stream;
use jsonrpsee::core::async_trait;
use race_core::error::Result;
use race_core::types::{
    AssignRecipientParams, CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, ServeParams, SettleResult, UnregisterGameParams, VoteParams
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
use tokio_stream::StreamExt;
use std::pin::Pin;
use std::time::Duration;
use tracing::{error, info};

const DEFAULT_RETRY_INTERVAL: u64 = 10;
const DEFAULT_RESUB_INTERVAL: u64 = 5;

pub struct WrappedTransport {
    pub(crate) inner: Box<dyn TransportT>,
    retry_interval: u64,
    resub_interval: u64,
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
        Ok(Self { inner: transport, retry_interval: DEFAULT_RETRY_INTERVAL, resub_interval: DEFAULT_RESUB_INTERVAL })
    }
}

#[async_trait]
impl TransportT for WrappedTransport {
    async fn subscribe_game_account<'a>(&'a self, addr: &'a str) -> Result<Pin<Box<dyn Stream<Item = Result<GameAccount>> + Send + 'a>>> {
        let interval = self.resub_interval;
        Ok(Box::pin(stream! {
            let sub = self.inner.subscribe_game_account(addr).await;

            let mut sub = match sub {
                Ok(sub) => sub,
                Err(e) => {
                    return yield Err(e);
                }
            };

            loop {
                let item = sub.next().await;
                match item {
                    Some(Ok(item)) => {
                        yield Ok(item);
                    },
                    Some(Err(e)) => {
                        return yield Err(e);
                    }
                    None => {
                        info!("Restart subscription after {} seconds", interval);
                        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                        let new_sub = self.inner.subscribe_game_account(addr).await;
                        sub = match new_sub {
                            Ok(new_sub) => new_sub,
                            Err(e) => {
                                return yield Err(e);
                            }
                        };
                    }
                }
            }
        }))
    }

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

    async fn get_game_account(&self, addr: &str) -> Result<Option<GameAccount>> {
        self.inner.get_game_account(addr).await
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
    async fn settle_game(&self, params: SettleParams) -> Result<SettleResult> {
        let mut curr_settle_version: Option<u64> = None;
        loop {
            let game_account = self
                .inner
                .get_game_account(&params.addr)
                .await;
            if let Ok(Some(game_account)) = game_account {
                // We got an old state, which has a smaller `settle_version`
                if game_account.settle_version < params.settle_version {
                    error!(
                        "Got invalid settle_version: {} != {}, will retry in {} secs",
                        game_account.settle_version, params.settle_version, self.retry_interval
                    );
                    tokio::time::sleep(Duration::from_secs(self.retry_interval)).await;
                    continue;
                }
                // The `settle_version` had been bumped, which
                // indicates the transaction was succeed

                //NOTE: The transaction may success with error result
                // due to unstable network
                if curr_settle_version.is_some_and(|v| v < game_account.settle_version) {
                    return Ok(SettleResult {
                        signature: "".into(),
                        game_account,
                    });
                }
                curr_settle_version = Some(game_account.settle_version);
                match self.inner.settle_game(params.clone()).await {
                    Ok(rst) => return Ok(rst),
                    Err(e) => {
                        error!(
                            "Error in settlement: {:?}, will retry in {} secs",
                            e, self.retry_interval
                        );
                        tokio::time::sleep(Duration::from_secs(self.retry_interval)).await;
                        continue;
                    }
                }
            } else {
                error!(
                    "Error in settlement due to unable to get game account {}, will retry in {} secs",
                    params.addr,
                    self.retry_interval
                );
                tokio::time::sleep(Duration::from_millis(self.retry_interval)).await;
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
    use race_core::checkpoint::CheckpointOnChain;
    use race_test::prelude::{test_game_addr, DummyTransport, TestGameAccountBuilder};

    use super::*;

    #[tokio::test]
    async fn test_settle_without_retry() -> anyhow::Result<()> {
        let t = DummyTransport::default();
        let ga0 = TestGameAccountBuilder::new().build();
        let mut ga1 = TestGameAccountBuilder::new().build();
        ga1.settle_version = 1;
        t.simulate_states(vec![ga0, ga1]);
        let wt = WrappedTransport { inner: Box::new(t), retry_interval: 1 };
        let r = wt
            .settle_game(SettleParams {
                addr: test_game_addr(),
                settles: vec![],
                transfers: vec![],
                checkpoint: CheckpointOnChain::default(),
                settle_version: 1,
                next_settle_version: 2,
            })
            .await;

        assert_eq!(r, Ok("".to_string()));
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
        let wt = WrappedTransport { inner: Box::new(t), retry_interval: 1 };
        let r = wt
            .settle_game(SettleParams {
                addr: test_game_addr(),
                transfers: vec![],
                settles: vec![],
                checkpoint: CheckpointOnChain::default(),
                settle_version: 0,
                next_settle_version: 2,
            })
            .await;

        assert_eq!(r, Ok("".to_string()));
        Ok(())
    }
}

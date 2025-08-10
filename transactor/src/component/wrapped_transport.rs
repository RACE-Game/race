//! Wrapped transport, which support retry

use async_stream::stream;
use futures::Stream;
use jsonrpsee::core::async_trait;
use race_core::error::{Error, Result};
use race_core::types::{
    AddRecipientSlotParams, AssignRecipientParams, CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams, DepositStatus, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RejectDepositsParams, RejectDepositsResult, ServeParams, SettleResult, UnregisterGameParams, VoteParams
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
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt;
use tracing::{error, info};

const DEFAULT_RETRY_INTERVAL: u64 = 10;
const DEFAULT_RESUB_INTERVAL: u64 = 5;

pub struct BundleCache {
    bundles: Arc<Mutex<HashMap<String, GameBundle>>>,
}

impl BundleCache {
    pub fn new() -> Self {
        Self {
            bundles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_cache(&self, bundle: GameBundle) {
        self.bundles.lock().await.insert(bundle.addr.clone(), bundle);
    }

    pub async fn get_cache(&self, addr: &str) -> Option<GameBundle> {
        self.bundles.lock().await.get(addr).map(ToOwned::to_owned)
    }
}

pub struct WrappedTransport {
    pub(crate) inner: Box<dyn TransportT>,
    retry_interval: u64,
    resub_interval: u64,
    bundle_cache: BundleCache,
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
        let bundle_cache = BundleCache::new();
        Ok(Self {
            inner: transport,
            bundle_cache,
            retry_interval: DEFAULT_RETRY_INTERVAL,
            resub_interval: DEFAULT_RESUB_INTERVAL,
        })
    }
}

#[async_trait]
impl TransportT for WrappedTransport {
    async fn subscribe_game_account<'a>(
        &'a self,
        addr: &'a str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GameAccount>> + Send + 'a>>> {
        let interval = self.resub_interval;
        Ok(Box::pin(stream! {
            let sub = self.inner.subscribe_game_account(addr).await;

            let mut sub = match sub {
                Ok(sub) => sub,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            loop {
                let item = sub.next().await;
                match item {
                    Some(Ok(item)) => {
                        yield Ok(item);
                    },
                    Some(Err(e)) => {
                        error!("An error occurred in game account subscription, quit sub loop");
                        yield Err(e);
                        return;
                    }
                    None => {
                        sub = loop {
                            info!("Restart subscription after {} seconds", interval);
                            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                            let new_sub = self.inner.subscribe_game_account(addr).await;
                            match new_sub {
                                Ok(new_sub) => break new_sub,
                                Err(e) => {
                                    error!("Subscribe game account err: {}", e.to_string());
                                    continue;
                                }
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
        if let Some(game_bundle) = self.bundle_cache.get_cache(addr).await {
            info!("Use game bundle from cache: {}", addr);
            return Ok(Some(game_bundle));
        } else {
            let r = self.inner.get_game_bundle(addr).await;
            if let Ok(Some(game_bundle)) = r {
                self.bundle_cache.add_cache(game_bundle.clone()).await;
                return Ok(Some(game_bundle))
            } else {
                return r
            }
        }
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
            let game_account = self.inner.get_game_account(&params.addr).await;
            if let Ok(Some(game_account)) = game_account {
                // We got an old state, which has a smaller settle_version.
                // It means either the previous settle was failed, or RPC gave a outdated account.
                // We should retry the query to get the latest account.
                if game_account.settle_version < params.settle_version {
                    error!(
                        "Got invalid settle_version: on-chain version {} != required version {}, will retry in {} secs",
                        game_account.settle_version, params.settle_version, self.retry_interval
                    );
                    tokio::time::sleep(Duration::from_secs(self.retry_interval)).await;
                    continue;
                }
                // We got a settle_version which is greater than the current one.  It means that the
                // settle_version has been bumped. There's no need to retry.
                // NOTE: The transaction // may success with error result due to unstable network
                if curr_settle_version.is_some_and(|v| v < game_account.settle_version) {
                    return Ok(SettleResult {
                        signature: "".into(),
                        game_account,
                    });
                }
                curr_settle_version = Some(game_account.settle_version);

                match self.inner.settle_game(params.clone()).await {
                    Ok(rst) => {
                        info!("Settlement succeed, signature: {}", rst.signature);
                        return Ok(rst);
                    }
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
                tokio::time::sleep(Duration::from_secs(self.retry_interval)).await;
                continue;
            }
        }
    }

    async fn reject_deposits(&self, params: RejectDepositsParams) -> Result<RejectDepositsResult> {
        let Some(max_access_version) = params.reject_deposits.iter().max() else {
            return Err(Error::EmptyRejectDeposits);
        };

        loop {
            let game_account = self.inner.get_game_account(&params.addr).await;

            if let Ok(Some(game_account)) = game_account {
                // We got an old state, because the access_version is too small
                if game_account.access_version < *max_access_version {
                    error!(
                        "Got invalid access_version: {} < {}, will retry in {} secs",
                        game_account.access_version, max_access_version, self.retry_interval
                    );
                    tokio::time::sleep(Duration::from_secs(self.retry_interval)).await;
                    continue;
                }

                // If we see the deposits are marked as rejected/refunded, we should skip

                for d in game_account.deposits.iter() {
                    if params.reject_deposits.iter().any(|rd| {
                        *rd == d.access_version
                            && (d.status == DepositStatus::Rejected
                                || d.status == DepositStatus::Refunded)
                    }) {
                        return Ok(RejectDepositsResult {
                            signature: "".to_string(),
                        });
                    }
                }

                match self.inner.reject_deposits(params.clone()).await {
                    Ok(rst) => return Ok(rst),
                    Err(e) => {
                        error!(
                            "Error in reject_deposits: {:?}, will retry in {} secs",
                            e, self.retry_interval
                        );
                        tokio::time::sleep(Duration::from_secs(self.retry_interval)).await;
                        continue;
                    }
                }
            } else {
                error!(
                    "Error in reject_deposits due to unable to get game account {}, will retry in {} secs",
                    params.addr,
                    self.retry_interval
                );
                tokio::time::sleep(Duration::from_secs(self.retry_interval)).await;
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

    async fn add_recipient_slot(&self, params: AddRecipientSlotParams) -> Result<String> {
        self.inner.add_recipient_slot(params).await
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
        let wt = WrappedTransport {
            inner: Box::new(t),
            retry_interval: 1,
            resub_interval: 1,
            bundle_cache: BundleCache::new(),
        };
        let r = wt
            .settle_game(SettleParams {
                addr: test_game_addr(),
                settles: vec![],
                transfer: None,
                checkpoint: CheckpointOnChain::default(),
                settle_version: 1,
                access_version: 1,
                accept_deposits: vec![],
                awards: vec![],
                entry_lock: None,
                next_settle_version: 2,
            })
            .await;

        assert_eq!(r.unwrap().signature, "".to_string());
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
        let wt = WrappedTransport {
            inner: Box::new(t),
            retry_interval: 1,
            resub_interval: 1,
            bundle_cache: BundleCache::new(),
        };
        let r = wt
            .settle_game(SettleParams {
                addr: test_game_addr(),
                transfer: None,
                settles: vec![],
                checkpoint: CheckpointOnChain::default(),
                settle_version: 0,
                access_version: 1,
                accept_deposits: vec![],
                awards: vec![],
                entry_lock: None,
                next_settle_version: 2,
            })
            .await;

        assert_eq!(r.unwrap().signature, "".to_string());
        Ok(())
    }
}

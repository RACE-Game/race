use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use race_api::error::Error;
use race_core::storage::StorageT;
use race_core::types::{GameAccount, SaveCheckpointParams, SettleParams, SettleResult, TxState};
use tokio::select;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::component::common::Component;
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;
use race_core::transport::TransportT;

use super::ComponentEnv;
use super::common::PipelinePorts;

const MAX_PENDING_TXS: usize = 10;

/// Squash two settles into one.
fn squash_settles(mut prev: SettleParams, next: SettleParams) -> SettleParams {
    let SettleParams {
        addr,
        settles,
        transfers,
        checkpoint,
        entry_lock,
        ..
    } = next;
    prev.settles.extend(settles);
    prev.transfers.extend(transfers);
    let entry_lock = if entry_lock.is_none() {
        prev.entry_lock
    } else {
        entry_lock
    };
    SettleParams {
        addr,
        settles: prev.settles,
        transfers: prev.transfers,
        // Use the latest checkpoint
        checkpoint,
        // Use the old settle_version
        settle_version: prev.settle_version,
        next_settle_version: prev.next_settle_version + 1,
        entry_lock,
    }
}

/// Read at most `MAX_PENDING_TXS` settle events from channel.
async fn read_settle_params(rx: &mut mpsc::Receiver<SettleParams>) -> Vec<SettleParams> {
    let mut v = vec![];
    let mut cnt = 0;

    loop {
        if cnt == MAX_PENDING_TXS {
            break;
        }

        select! {
            p = rx.recv() => {
                if let Some(p) = p {
                    cnt += 1;
                    v.push(p);
                } else {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                if v.is_empty() {
                    continue;
                } else {
                    break;
                }
            }
        }
    }

    v
}

pub struct SubmitterContext {
    addr: String,
    transport: Arc<dyn TransportT>,
    storage: Arc<dyn StorageT>,
}

pub struct Submitter {}

impl Submitter {
    pub fn init(
        game_account: &GameAccount,
        transport: Arc<dyn TransportT>,
        storage: Arc<dyn StorageT>,
    ) -> (Self, SubmitterContext) {
        (
            Self {},
            SubmitterContext {
                addr: game_account.addr.clone(),
                transport,
                storage,
            },
        )
    }
}

#[async_trait]
impl Component<PipelinePorts, SubmitterContext> for Submitter {
    fn name() -> &'static str {
        "Submitter"
    }

    async fn run(mut ports: PipelinePorts, ctx: SubmitterContext, env: ComponentEnv) -> CloseReason {
        let (queue_tx, mut queue_rx) = mpsc::channel::<SettleParams>(32);
        let p = ports.clone_as_producer();
        // Start a task to handle settlements
        // Prevent the blocking from pending transactions
        let join_handle = tokio::spawn(async move {
            loop {
                let ps = read_settle_params(&mut queue_rx).await;
                if let Some(params) = ps.into_iter().reduce(squash_settles) {
                    let settle_version = params.settle_version;
                    let res = ctx.transport.settle_game(params).await;
                    match res {
                        Ok(SettleResult{ signature, game_account }) => {
                            let tx_state = TxState::SettleSucceed {
                                signature: if signature.is_empty() {
                                    None
                                } else {
                                    Some(signature)
                                },
                                settle_version,
                            };
                            p.send(EventFrame::TxState { tx_state }).await;

                            let mut new_deposits = vec![];
                            let GameAccount{ transactor_addr, deposits, access_version, .. } = game_account;
                            for d in deposits {
                                if d.settle_version == game_account.settle_version {
                                    new_deposits.push(d);
                                }
                            }
                            if !new_deposits.is_empty() {
                                let sync = EventFrame::Sync{
                                    new_players: vec![],
                                    new_servers: vec![],
                                    new_deposits,
                                    transactor_addr: transactor_addr.unwrap_or_default(),
                                    access_version,
                                };
                                p.send(sync).await;
                            }
                        }
                        Err(e) => {
                            return CloseReason::Fault(e);
                        }
                    }
                } else {
                    break;
                }
            }
            CloseReason::Complete
        });

        while let Some(event) = ports.recv().await {
            match event {
                EventFrame::Checkpoint {
                    settles,
                    transfers,
                    checkpoint,
                    settle_version,
                    previous_settle_version,
                    entry_lock,
                    ..
                } => {
                    let checkpoint_onchain = checkpoint.derive_onchain_part();
                    let checkpoint_offchain = checkpoint.derive_offchain_part();

                    info!("{} Submitter save checkpoint to storage, settle_version = {}",
                        env.log_prefix, settle_version
                    );
                    let save_checkpoint_result = ctx.storage.save_checkpoint(SaveCheckpointParams {
                        game_addr: ctx.addr.clone(),
                        settle_version,
                        checkpoint: checkpoint_offchain,
                    }).await;

                    if let Err(e) = save_checkpoint_result {
                        error!("{} Submitter failed to save checkpoint offchain: {}",
                            env.log_prefix, e.to_string());
                        break;
                    }

                    let res = queue_tx
                        .send(SettleParams {
                            addr: ctx.addr.clone(),
                            settles,
                            transfers,
                            checkpoint: checkpoint_onchain,
                            settle_version: previous_settle_version,
                            next_settle_version: settle_version,
                            entry_lock,
                        })
                        .await;
                    if let Err(e) = res {
                        error!(
                            "{} Submitter failed to send settle to task queue: {}",
                            env.log_prefix,
                            e.to_string()
                        );
                    }
                }
                EventFrame::Shutdown => {
                    drop(queue_tx);
                    break;
                }
                _ => (),
            }
        }

        join_handle.await.unwrap_or_else(|e| {
            CloseReason::Fault(Error::InternalError(format!(
                "Submitter await join handle error: {}",
                e
            )))
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use race_core::{checkpoint::Checkpoint, types::SettleWithAddr};
    use race_local_db::LocalDbStorage;
    use race_test::prelude::*;

    #[tokio::test]
    async fn test_submit_settle() {
        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let mut charlie = TestClient::player("charlie");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&mut alice, 100)
            .add_player(&mut bob, 100)
            .add_player(&mut charlie, 100)
            .build();
        let transport = Arc::new(DummyTransport::default());
        let storage = Arc::new(LocalDbStorage::try_new_mem().unwrap());
        let (submitter, ctx) = Submitter::init(&game_account, transport.clone(), storage.clone());

        let settles = vec![
            SettleWithAddr::sub("alice", 50),
            SettleWithAddr::add("alice", 20),
            SettleWithAddr::add("alice", 20),
            SettleWithAddr::sub("alice", 40),
            SettleWithAddr::add("bob", 50),
            SettleWithAddr::sub("bob", 20),
            SettleWithAddr::sub("bob", 20),
            SettleWithAddr::sub("bob", 20),
            SettleWithAddr::add("bob", 30),
            SettleWithAddr::eject("charlie"),
        ];

        let event_frame = EventFrame::Checkpoint {
            settles: settles.clone(),
            transfers: vec![],
            checkpoint: Checkpoint::default(),
            settle_version: 1,
            previous_settle_version: 0,
            access_version: 1,
            state_sha: "".into(),
        };
        let handle = submitter.start("TEST", ctx);

        handle.send_unchecked(event_frame).await;
        handle.send_unchecked(EventFrame::Shutdown).await;
        handle.wait().await;

        assert_eq!(*transport.get_settles(), settles);
    }
}

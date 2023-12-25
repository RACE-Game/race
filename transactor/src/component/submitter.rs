use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use race_api::error::Error;
use race_core::types::{GameAccount, SettleParams};
use tokio::select;
use tokio::sync::mpsc;
use tracing::error;

use crate::component::common::{Component, ConsumerPorts};
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;
use race_core::transport::TransportT;

/// Squash two settles into one.
fn squash_settles(mut prev: SettleParams, next: SettleParams) -> SettleParams {
    let SettleParams {
        addr,
        settles,
        transfers,
        checkpoint,
        ..
    } = next;
    prev.settles.extend(settles);
    prev.transfers.extend(transfers);
    SettleParams {
        addr,
        settles: prev.settles,
        transfers: prev.transfers,
        // Use the latest checkpoint
        checkpoint,
        // Use the old settle_version
        settle_version: prev.settle_version,
        next_settle_version: prev.next_settle_version + 1,
    }
}

/// Read at most 3 settle events from channel.
async fn read_settle_params(rx: &mut mpsc::Receiver<SettleParams>) -> Vec<SettleParams> {
    let mut v = vec![];
    let mut cnt = 0;

    loop {
        if cnt == 3 {
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
}

pub struct Submitter {}

impl Submitter {
    pub fn init(
        game_account: &GameAccount,
        transport: Arc<dyn TransportT>,
    ) -> (Self, SubmitterContext) {
        (
            Self {},
            SubmitterContext {
                addr: game_account.addr.clone(),
                transport,
            },
        )
    }
}

#[async_trait]
impl Component<ConsumerPorts, SubmitterContext> for Submitter {
    fn name(&self) -> &str {
        "Submitter"
    }

    async fn run(mut ports: ConsumerPorts, ctx: SubmitterContext) -> CloseReason {
        let (queue_tx, mut queue_rx) = mpsc::channel::<SettleParams>(32);

        // Start a task to handle settlements
        // Prevent the blocking from pending transactions
        let join_handle = tokio::spawn(async move {
            loop {
                let ps = read_settle_params(&mut queue_rx).await;
                if let Some(params) = ps.into_iter().reduce(squash_settles) {
                    let res = ctx.transport.settle_game(params).await;
                    match res {
                        Ok(_) => (),
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
                EventFrame::Settle {
                    settles,
                    transfers,
                    checkpoint,
                    settle_version,
                } => {
                    let res = queue_tx.send(SettleParams {
                        addr: ctx.addr.clone(),
                        settles,
                        transfers,
                        checkpoint,
                        settle_version,
                        next_settle_version: settle_version + 1,
                    }).await;
                    if let Err(e) = res {
                        error!("Submitter failed to send settle to task queue: {}", e.to_string());
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
    use race_core::types::SettleWithAddr;
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
        let (submitter, ctx) = Submitter::init(&game_account, transport.clone());

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

        let event_frame = EventFrame::Settle {
            settles: settles.clone(),
            transfers: vec![],
            checkpoint: vec![],
            settle_version: 0,
        };
        let handle = submitter.start(ctx);

        handle.send_unchecked(event_frame).await;
        handle.send_unchecked(EventFrame::Shutdown).await;
        handle.wait().await;

        assert_eq!(*transport.get_settles(), settles);
    }
}

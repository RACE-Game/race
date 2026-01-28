/// The subscribers
/// Subscriber used to subscribe events from the transactor.
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::pin_mut;
use futures::StreamExt;
use race_core::types::BroadcastFrame;
use race_core::types::BroadcastSync;
use race_core::types::DepositStatus;
use race_core::types::VoteType;
use race_core::types::{GameAccount, ServerAccount};
use tokio::select;
use tracing::error;
use tracing::info;
use tracing::warn;

use race_transactor_frames::EventFrame;

use super::common::Component;
use super::common::PipelinePorts;
use super::ComponentEnv;
use super::{event_bus::CloseReason, RemoteConnection};

pub struct SubscriberContext {
    game_addr: String,
    #[allow(unused)]
    server_addr: String,
    transactor_addr: String,
    start_settle_version: u64,
    #[allow(unused)]
    init_game_account: GameAccount,
    connection: Arc<RemoteConnection>,
}

pub struct Subscriber {}

impl Subscriber {
    pub fn init(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        connection: Arc<RemoteConnection>,
    ) -> (Self, SubscriberContext) {
        (
            Self {},
            SubscriberContext {
                init_game_account: game_account.clone(),
                game_addr: game_account.addr.clone(),
                server_addr: server_account.addr.clone(),
                transactor_addr: game_account.transactor_addr.as_ref().unwrap().clone(),
                start_settle_version: game_account.settle_version,
                connection,
            },
        )
    }
}

async fn handle_frame(frame: BroadcastFrame, ports: &mut PipelinePorts, env: &ComponentEnv) -> Pin<Box<Option<CloseReason>>> {
    let ret = match frame {
        // Forward event to event bus
        BroadcastFrame::Event {
            event, timestamp, ..
        } => {
            info!("{} Receive event: {}", env.log_prefix, event);
            if let Err(e) = ports
                .try_send(EventFrame::SendServerEvent { event, timestamp })
                .await
            {
                error!("Send server event error: {}", e);
                Some(CloseReason::Complete)
            } else {
                None
            }
        }

        BroadcastFrame::Sync { sync } => {
            let BroadcastSync {
                new_players,
                new_servers,
                mut new_deposits,
                access_version,
                transactor_addr,
            } = sync;
            info!(
                "{} Receive Sync broadcast, new_players: {:?}, new_servers: {:?}",
                env.log_prefix, new_players, new_servers
            );

            new_deposits.retain(|d| d.status == DepositStatus::Pending);

            if let Err(e) = ports
                .try_send(EventFrame::Sync {
                    new_players,
                    new_servers,
                    new_deposits,
                    transactor_addr,
                    access_version,
                })
                .await
            {
                error!("{} Send update node error: {}", env.log_prefix, e);
                Some(CloseReason::Complete)
            } else {
                None
            }
        }

        BroadcastFrame::Message { .. } => {
            None
        }
        BroadcastFrame::TxState { .. } => {
            None
        }
        BroadcastFrame::Backlogs { backlogs, .. } => {
            info!(
                "{} Receive event backlogs: {}",
                env.log_prefix,
                backlogs.len()
            );
            let mut r = None;
            for backlog_frame in *backlogs {
                if let Some(close_reason) = *Pin::into_inner(Box::pin(handle_frame(backlog_frame, ports, &env)).await) {
                    r = Some(close_reason.clone());
                }
            };
            r
        }
    };
    Box::pin(ret)
}

#[async_trait]
impl Component<PipelinePorts, SubscriberContext> for Subscriber {
    fn name() -> &'static str {
        "Subscriber"
    }

    async fn run(
        mut ports: PipelinePorts,
        ctx: SubscriberContext,
        env: ComponentEnv,
    ) -> CloseReason {
        let SubscriberContext {
            game_addr,
            server_addr: _,
            transactor_addr,
            start_settle_version,
            connection,
            ..
        } = ctx;

        // Wait one RecoverCheckpointWithCredentials frame before connecting transactor.
        // Make sure we have a prepared handler state before the first event.
        loop {
            let frame = ports.recv().await;
            if matches!(frame, Some(EventFrame::RecoverCheckpointWithCredentials { .. })) {
                break;
            }
        }

        let mut retries = 0;
        let sub = loop {
            match connection
                .subscribe_events(&game_addr, start_settle_version)
                .await
            {
                Ok(sub) => break sub,
                Err(e) => {
                    if retries == 3 {
                        error!(
                            "{} Failed to subscribe events: {}. Vote on the transactor {} has dropped",
                            env.log_prefix, e, transactor_addr
                        );

                        ports
                            .send(EventFrame::Vote {
                                votee: transactor_addr,
                                vote_type: VoteType::ServerVoteTransactorDropOff,
                            })
                            .await;

                        warn!("{} Shutdown subscriber", env.log_prefix);
                        return CloseReason::Complete;
                    } else {
                        error!(
                            "{} Failed to subscribe events: {}, will retry",
                            env.log_prefix, e
                        );
                        retries += 1;
                        continue;
                    }
                }
            }
        };

        info!("{} Subscription established", env.log_prefix);
        pin_mut!(sub);

        loop {
            select! {
                event_frame = ports.recv() => {
                    match event_frame {
                        Some(EventFrame::Shutdown) => {
                            info!("{} Stopped", env.log_prefix);
                            return CloseReason::Complete;
                        }
                        _ => ()
                    }
                }

                frame = sub.next() => {
                    let Some(frame) = frame else {
                        break;
                    };

                    if let Some(close_reason) = *Pin::into_inner(handle_frame(frame, &mut ports, &env).await) {
                        return close_reason;
                    }
                }
            }
        }

        warn!("{} Vote for disconnecting", env.log_prefix);
        ports
            .send(EventFrame::Vote {
                votee: transactor_addr,
                vote_type: VoteType::ServerVoteTransactorDropOff,
            })
            .await;

        return CloseReason::Complete;
    }
}

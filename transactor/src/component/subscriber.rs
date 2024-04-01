/// The subscribers
/// Subscriber used to subscribe events from the transactor.
use std::sync::Arc;

use async_trait::async_trait;
use futures::pin_mut;
use futures::StreamExt;
use race_core::types::BroadcastFrame;
use race_core::types::BroadcastSync;
use race_core::types::VoteType;
use race_core::types::{GameAccount, ServerAccount};
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::frame::EventFrame;

use super::common::{Component, ProducerPorts};
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

#[async_trait]
impl Component<ProducerPorts, SubscriberContext> for Subscriber {
    fn name() -> &'static str {
        "Subscriber"
    }

    async fn run(ports: ProducerPorts, ctx: SubscriberContext, env: ComponentEnv) -> CloseReason {
        let SubscriberContext {
            game_addr,
            server_addr: _,
            transactor_addr,
            start_settle_version,
            connection,
            ..
        } = ctx;

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

        while let Some(frame) = sub.next().await {
            match frame {
                // Forward event to event bus
                BroadcastFrame::Event { event, .. } => {
                    info!("{} Receive event: {}", env.log_prefix, event);
                    if let Err(e) = ports.try_send(EventFrame::SendServerEvent { event }).await {
                        error!("Send server event error: {}", e);
                        break;
                    }
                }

                BroadcastFrame::Sync { sync } => {
                    let BroadcastSync {
                        new_players,
                        new_servers,
                        access_version,
                        transactor_addr,
                    } = sync;
                    info!(
                        "{} Receive Sync broadcast, new_players: {:?}, new_servers: {:?}",
                        env.log_prefix, new_players, new_servers
                    );
                    if let Err(e) = ports
                        .try_send(EventFrame::Sync {
                            new_players,
                            new_servers,
                            transactor_addr,
                            access_version,
                        })
                        .await
                    {
                        error!("{} Send update node error: {}", env.log_prefix, e);
                        break;
                    }
                }

                BroadcastFrame::Message { .. } => {
                    // Dropped
                }
                BroadcastFrame::TxState { .. } => {
                    // Dropped
                }
                BroadcastFrame::EndOfHistory => {
                    // Dropped
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

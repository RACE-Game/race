/// The subscribers
/// Subscriber used to subscribe events from the transactor.
use std::sync::Arc;

use futures::pin_mut;
use futures::StreamExt;
use race_core::transport::TransportT;
use race_core::types::BroadcastFrame;
use race_core::types::VoteParams;
use race_core::types::VoteType;
use race_core::types::{GameAccount, ServerAccount};
use tokio::sync::{mpsc, oneshot};
use tracing::error;
use tracing::info;

use crate::frame::EventFrame;

use super::{event_bus::CloseReason, Attachable, Component, Named, RemoteConnection};

pub(crate) struct SubscriberContext {
    game_addr: String,
    server_addr: String,
    transactor_addr: String,
    start_settle_version: u64,
    output_tx: mpsc::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    transport: Arc<dyn TransportT>,
    connection: Arc<RemoteConnection>,
}

pub struct Subscriber {
    output_rx: Option<mpsc::Receiver<EventFrame>>,
    closed_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<SubscriberContext>,
}

impl Attachable for Subscriber {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        None
    }

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        let mut ret = None;
        std::mem::swap(&mut ret, &mut self.output_rx);
        ret
    }
}

impl Named for Subscriber {
    fn name<'a>(&self) -> &'a str {
        "Subscriber"
    }
}

impl Component<SubscriberContext> for Subscriber {
    fn run(&mut self, ctx: SubscriberContext) {
        tokio::spawn(async move {
            let SubscriberContext {
                game_addr,
                server_addr,
                transactor_addr,
                start_settle_version,
                output_tx,
                closed_tx,
                transport,
                connection,
            } = ctx;

            let mut retries = 0;
            let sub = loop {
                match connection
                    .subscribe_events(&game_addr, &server_addr, start_settle_version)
                    .await
                {
                    Ok(sub) => break sub,
                    Err(e) => {
                        if retries == 5 {
                            error!(
                                "Failed to subscribe events: {}. Vote on the transactor {} has dropped",
                                e,
                                transactor_addr
                            );
                            transport
                                .vote(VoteParams {
                                    game_addr,
                                    vote_type: VoteType::ServerVoteTransactorDropOff,
                                    voter_addr: server_addr,
                                    votee_addr: transactor_addr,
                                })
                                .await
                                .unwrap();
                            return;
                        } else {
                            error!("Failed to subscribe events: {}, will retry", e);
                            retries += 1;
                            continue;
                        }
                    }
                }
            };

            pin_mut!(sub);

            while let Some(frame) = sub.next().await {
                info!("Subscriber received: {}", frame);
                let BroadcastFrame { event, .. } = frame;
                let r = output_tx.send(EventFrame::SendServerEvent { event }).await;
                if let Err(e) = r {
                    error!("Failed to send event, error: {:?}", e);
                }
            }

            if let Err(e) = closed_tx.send(CloseReason::Complete) {
                error!("Subscriber: Failed to close: {:?}", e);
            }
        });
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<SubscriberContext> {
        &mut self.ctx
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.closed_rx
    }
}

impl Subscriber {
    pub fn new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        transport: Arc<dyn TransportT>,
        connection: Arc<RemoteConnection>,
    ) -> Self {
        let start_settle_version = game_account.settle_version;
        let (output_tx, output_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = SubscriberContext {
            game_addr: game_account.addr.to_owned(),
            server_addr: server_account.addr.to_owned(),
            transactor_addr: game_account.transactor_addr.as_ref().unwrap().to_owned(),
            start_settle_version,
            output_tx,
            closed_tx,
            transport,
            connection,
        };
        Self {
            output_rx: Some(output_rx),
            closed_rx,
            ctx: Some(ctx),
        }
    }
}

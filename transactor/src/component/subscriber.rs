/// The subscribers
/// Subscriber used to subscribe events from the transactor.
use std::sync::Arc;

use futures::pin_mut;
use futures::StreamExt;
use race_core::types::BroadcastFrame;
use race_core::types::{GameAccount, ServerAccount};
use tokio::sync::{mpsc, oneshot};
use tracing::error;

use crate::frame::EventFrame;

use super::{event_bus::CloseReason, Attachable, Component, Named, RemoteConnection};

pub(crate) struct SubscriberContext {
    game_addr: String,
    server_addr: String,
    start_settle_version: u64,
    output_tx: mpsc::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
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
                start_settle_version,
                output_tx,
                closed_tx,
                connection,
            } = ctx;

            let sub = connection
                .subscribe_events(&game_addr, &server_addr, start_settle_version)
                .await
                .expect("Failed to subscribe event");

            pin_mut!(sub);

            while let Some(frame) = sub.next().await {
                let BroadcastFrame { event, .. } = frame;
                output_tx.send(EventFrame::SendServerEvent { event }).await.ok();
            }

            if let Err(e) = closed_tx.send(CloseReason::Complete) {
                error!("Failed to close: {:?}", e);
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
        connection: Arc<RemoteConnection>,
    ) -> Self {
        let start_settle_version = game_account.settle_version;
        let (output_tx, output_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = SubscriberContext {
            game_addr: game_account.addr.to_owned(),
            server_addr: server_account.addr.to_owned(),
            start_settle_version,
            output_tx,
            closed_tx,
            connection,
        };
        Self {
            output_rx: Some(output_rx),
            closed_rx,
            ctx: Some(ctx),
        }
    }
}

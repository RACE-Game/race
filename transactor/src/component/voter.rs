//! The component to make voting transaction.  It happens when
//! transactor is considered dropped off.  A shutdown event will be
//! sent when vote is sent.

use std::{sync::Arc, time::Duration};

use race_core::{
    error::Error,
    transport::TransportT,
    types::{GameAccount, ServerAccount, VoteParams},
};
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::frame::EventFrame;

use super::{event_bus::CloseReason, Attachable, Component, Named};

pub(crate) struct VoterContext {
    game_addr: String,
    server_addr: String,
    transport: Arc<dyn TransportT>,
    output_tx: mpsc::Sender<EventFrame>,
    input_rx: mpsc::Receiver<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
}

pub struct Voter {
    output_rx: Option<mpsc::Receiver<EventFrame>>,
    input_tx: mpsc::Sender<EventFrame>,
    closed_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<VoterContext>,
}

impl Attachable for Voter {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        let mut ret = None;
        std::mem::swap(&mut ret, &mut self.output_rx);
        ret
    }
}

impl Named for Voter {
    fn name<'a>(&self) -> &'a str {
        "Voter"
    }
}

impl Component<VoterContext> for Voter {
    fn run(&mut self, mut ctx: VoterContext) {
        tokio::spawn(async move {
            while let Some(frame) = ctx.input_rx.recv().await {
                match frame {
                    EventFrame::Vote { votee, vote_type } => {
                        let params = VoteParams {
                            game_addr: ctx.game_addr.clone(),
                            vote_type,
                            voter_addr: ctx.server_addr.clone(),
                            votee_addr: votee,
                        };
                        // We keep retrying until success.
                        loop {
                            let r = ctx.transport.vote(params.clone()).await;
                            match r {
                                Ok(_) | Err(Error::DuplicateVote) => {
                                    info!("Vote sent");
                                    ctx.output_tx.send(EventFrame::Shutdown).await.unwrap();
                                    break;
                                }
                                Err(e) => {
                                    warn!("An error occurred in vote: {:?}, will retry.", e);
                                    tokio::time::sleep(Duration::from_secs(3)).await;
                                }
                            }
                        }
                    }
                    EventFrame::Shutdown => {
                        warn!("Shutdown voter");
                        break;
                    }
                    _ => (),
                }
            }

            if let Err(e) = ctx.closed_tx.send(CloseReason::Complete) {
                warn!("Failed to send close reason: {:?}", e);
            }
        });
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<VoterContext> {
        &mut self.ctx
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.closed_rx
    }
}

impl Voter {
    pub fn new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        transport: Arc<dyn TransportT>,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
        let (output_tx, output_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();

        let ctx = VoterContext {
            game_addr: game_account.addr.clone(),
            server_addr: server_account.addr.clone(),
            transport,
            input_rx,
            output_tx,
            closed_tx,
        };
        Self {
            output_rx: Some(output_rx),
            input_tx,
            closed_rx,
            ctx: Some(ctx),
        }
    }
}

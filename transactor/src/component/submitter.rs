use std::sync::Arc;

use race_core::types::{GameAccount, SettleParams};
use tokio::sync::{mpsc, oneshot};

use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};
use crate::frame::EventFrame;
use race_core::transport::TransportT;

pub(crate) struct SubmitterContext {
    addr: String,
    input_rx: mpsc::Receiver<EventFrame>,
    close_tx: oneshot::Sender<CloseReason>,
    transport: Arc<dyn TransportT>,
}

/// A component that submits events to blockchain
/// To construct a submitter, a chain adapter is required.
pub struct Submitter {
    input_tx: mpsc::Sender<EventFrame>,
    close_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<SubmitterContext>,
}

impl Named for Submitter {
    fn name<'a>(&self) -> &'a str {
        "Submitter"
    }
}

impl Attachable for Submitter {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        None
    }
}

impl Component<SubmitterContext> for Submitter {
    fn run(&mut self, mut ctx: SubmitterContext) {
        tokio::spawn(async move {
            while let Some(event) = ctx.input_rx.recv().await {
                match event {
                    EventFrame::Settle { settles } => {

                        // The wrapped transport will only return when the transaction is succeed.
                        // So here we assume the settle version is updated.
                        // The new settle_version equals to the old plus 1;
                        let res = ctx
                            .transport
                            .settle_game(SettleParams {
                                addr: ctx.addr.clone(),
                                settles,
                            })
                            .await;

                        match res {
                            Ok(_) => {

                            },
                            Err(e) => {
                                ctx.close_tx.send(CloseReason::Fault(e)).unwrap();
                                return;
                            }
                        }
                    }
                    EventFrame::Shutdown => {
                        break;
                    }
                    _ => (),
                }
            }
            ctx.close_tx.send(CloseReason::Complete).unwrap();
        });
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.close_rx
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<SubmitterContext> {
        &mut self.ctx
    }
}

impl Submitter {
    pub fn new(transport: Arc<dyn TransportT>, game_account: GameAccount) -> Self {
        let (input_tx, input_rx) = mpsc::channel(32);
        let (close_tx, close_rx) = oneshot::channel();
        let ctx = Some(SubmitterContext {
            addr: game_account.addr.clone(),
            input_rx,
            close_tx,
            transport,
        });
        Self {
            input_tx,
            close_rx,
            ctx,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use race_core::types::{Settle, SettleParams};
    use race_test::*;

    #[tokio::test]
    async fn test_submit_settle() {
        let game_account = TestGameAccountBuilder::default().add_players(2).build();
        let transport = Arc::new(DummyTransport::default());
        let mut submitter = Submitter::new(transport.clone(), game_account);
        let settles = vec![Settle::add("Alice", 100)];
        let params = SettleParams {
            addr: game_account_addr(),
            settles: settles.clone(),
        };
        let event_frame = EventFrame::Settle {
            settles: settles.clone(),
        };
        submitter.start();
        submitter.input_tx.send(event_frame).await.unwrap();
        submitter.input_tx.send(EventFrame::Shutdown).await.unwrap();
        submitter.closed().await.unwrap();
        assert_eq!(*transport.get_settles(), settles);
    }
}

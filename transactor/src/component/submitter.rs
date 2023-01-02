use std::sync::Arc;

use race_core::types::GameAccount;
use tokio::sync::{mpsc, oneshot, watch};

use crate::frame::EventFrame;
use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};
use race_core::transport::TransportT;

pub(crate) struct SubmitterContext {
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

    fn output(&self) -> Option<watch::Receiver<EventFrame>> {
        None
    }
}

impl Component<SubmitterContext> for Submitter {
    fn run(&mut self, mut ctx: SubmitterContext) {
        tokio::spawn(async move {
            while let Some(event) = ctx.input_rx.recv().await {
                match event {
                    EventFrame::Settle { addr, params } => {
                        ctx.transport.settle_game(params).await.unwrap();
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
    pub fn new(transport: Arc<dyn TransportT>, init_state: GameAccount) -> Self {
        let (input_tx, input_rx) = mpsc::channel(32);
        let (close_tx, close_rx) = oneshot::channel();
        let ctx = Some(SubmitterContext {
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
    use crate::utils::tests::game_account_with_empty_data;
    use race_core::types::{AssetChange, PlayerStatus, Settle, SettleParams};
    use race_transport::dummy::DummyTransport;
    use std::ops::Deref;

    #[tokio::test]
    async fn test_submit_settle() {
        let game_account = game_account_with_empty_data();
        let transport = Arc::new(DummyTransport::default());
        let mut submitter = Submitter::new(transport.clone(), game_account);
        let settles = vec![Settle::new(
            "Alice",
            PlayerStatus::Normal,
            AssetChange::Add,
            100,
        )];
        let params = SettleParams {
            addr: DummyTransport::mock_game_account_addr(),
            settles: settles.clone(),
        };
        let event_frame = EventFrame::Settle {
            addr: DummyTransport::mock_game_account_addr(),
            params,
        };
        submitter.start();
        submitter.input_tx.send(event_frame).await.unwrap();
        submitter.input_tx.send(EventFrame::Shutdown).await.unwrap();
        submitter.closed().await.unwrap();
        assert_eq!(transport.get_settles().deref(), &settles);
    }
}

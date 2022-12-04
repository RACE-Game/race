use std::sync::Arc;

use race_core::event::Event;
use race_core::types::{EventFrame, GameAccount};
use tokio::sync::{mpsc, oneshot, watch};

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

    use std::ops::Deref;
    use race_core::types::{SettleParams, PlayerStatus, AssetChange, Settle};
    use race_mock_transport::MockTransport;
    use super::*;

    #[tokio::test]
    async fn test_submit_settle() {
        let game_account = GameAccount {
            addr: "ACC ADDR".into(),
            game_addr: "GAME ADDR".into(),
            settle_serial: 0,
            access_serial: 0,
            players: vec![],
            data_len: 0,
            data: vec![],
        };
        let transport = Arc::new(MockTransport::default());
        let mut submitter = Submitter::new(transport.clone(), game_account);
        let settles = vec![Settle::new("Alice", PlayerStatus::Normal, AssetChange::Add, 100)];
        let params = SettleParams { addr: MockTransport::mock_game_account_addr(), settles: settles.clone() };
        let event_frame = EventFrame::Settle { addr: MockTransport::mock_game_account_addr(), params };
        submitter.start();
        submitter.input_tx.send(event_frame).await.unwrap();
        submitter.input_tx.send(EventFrame::Shutdown).await.unwrap();
        submitter.closed().await.unwrap();
        assert_eq!(transport.get_settles().deref(), &settles);
    }
}

use race_core::context::GameContext;
use race_core::error::Error;
use race_core::event::Event;
use tokio::sync::{mpsc, oneshot, watch};

use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};
use crate::component::wrapped_handler::WrappedHandler;
use race_core::types::{EventFrame, GameAccount};

pub struct EventLoopContext {
    input_rx: mpsc::Receiver<EventFrame>,
    output_tx: watch::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    handler: WrappedHandler,
    game_context: GameContext,
}

pub trait WrappedGameHandler: Send {
    fn init(&mut self, init_state: GameAccount) -> Result<(), Error>;

    fn handle_event(&mut self, event: EventFrame) -> Result<Vec<EventFrame>, Error>;
}

pub struct EventLoop {
    input_tx: mpsc::Sender<EventFrame>,
    output_rx: watch::Receiver<EventFrame>,
    closed_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<EventLoopContext>,
}

impl Named for EventLoop {
    fn name<'a>(&self) -> &'a str {
        "EventLoop"
    }
}

impl Attachable for EventLoop {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&self) -> Option<watch::Receiver<EventFrame>> {
        Some(self.output_rx.clone())
    }
}

impl Component<EventLoopContext> for EventLoop {
    fn run(&mut self, mut ctx: EventLoopContext) {
        tokio::spawn(async move {
            let mut handler = ctx.handler;
            let mut game_context = ctx.game_context;
            let output_tx = ctx.output_tx;
            while let Some(event_frame) = ctx.input_rx.recv().await {
                match event_frame {
                    EventFrame::PlayerJoined { addr, players } => {
                        for p in players.into_iter() {
                            if let Some(p) = p {
                                let event = Event::Join {
                                    player_addr: p.addr,
                                    balance: p.balance,
                                };
                                if let Ok(_) = handler.handle_event(&mut game_context, &event) {
                                    output_tx
                                        .send(EventFrame::Broadcast {
                                            addr: addr.clone(),
                                            state_json: game_context.get_handler_state_json().to_owned(),
                                            event,
                                        })
                                        .unwrap();
                                }
                            }
                        }
                    }
                    EventFrame::SendEvent { addr, event } => {
                        if let Ok(_) = handler.handle_event(&mut game_context, &event) {
                            output_tx
                                .send(EventFrame::Broadcast {
                                    addr,
                                    state_json: game_context.get_handler_state_json().to_owned(),
                                    event,
                                })
                                .unwrap();
                        }
                    }
                    EventFrame::Shutdown => {
                        ctx.closed_tx.send(CloseReason::Complete).unwrap();
                        break;
                    }
                    _ => (),
                }
            }
        });
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<EventLoopContext> {
        &mut self.ctx
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.closed_rx
    }
}

impl EventLoop {
    pub fn new(handler: WrappedHandler, game_context: GameContext) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
        let (output_tx, output_rx) = watch::channel(EventFrame::Empty);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = Some(EventLoopContext {
            input_rx,
            output_tx,
            closed_tx,
            handler,
            game_context,
        });
        Self {
            input_tx,
            output_rx,
            closed_rx,
            ctx,
        }
    }
}

#[cfg(test)]
mod tests {
    use race_core::types::Player;

    use super::*;

    #[tokio::test]
    async fn test_player_join() {
        let hdlr =
            WrappedHandler::load_by_path("../target/wasm32-unknown-unknown/release/race_example_minimal.wasm".into())
                .unwrap();
        let game_account = GameAccount {
            addr: "FAKE ADDR".into(),
            bundle_addr: "FAKE ADDR".into(),
            data_len: 4,
            data: vec![0, 0, 0, 42],
            ..Default::default()
        };
        let ctx = GameContext::new(&game_account);
        let mut event_loop = EventLoop::new(hdlr, ctx);
        event_loop.start();
        event_loop
            .input_tx
            .send(EventFrame::PlayerJoined {
                addr: "FAKE ADDR".into(),
                players: vec![Some(Player::new("Alice", 1000))],
            })
            .await
            .unwrap();
        if event_loop.output_rx.changed().await.is_ok() {
            let ef = event_loop.output_rx.borrow();
            assert_eq!(
                *ef,
                EventFrame::Broadcast {
                    addr: "FAKE ADDR".into(),
                    state_json: "{\"counter_value\":42,\"counter_player\":1}".into(),
                    event: Event::Join {
                        player_addr: "Alice".into(),
                        balance: 1000
                    }
                }
            )
        }
    }
}

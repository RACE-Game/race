use race_core::context::GameContext;
use race_core::error::Error;
use race_core::event::Event;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};
use crate::component::wrapped_handler::WrappedHandler;
use crate::frame::EventFrame;
use race_core::types::GameAccount;

pub struct EventLoopContext {
    input_rx: mpsc::Receiver<EventFrame>,
    output_tx: mpsc::Sender<EventFrame>,
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
    output_rx: Option<mpsc::Receiver<EventFrame>>,
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

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        let mut ret = None;
        std::mem::swap(&mut ret, &mut self.output_rx);
        ret
    }
}

async fn handle(
    handler: &mut WrappedHandler,
    game_context: &mut GameContext,
    event: Event,
    out: &mpsc::Sender<EventFrame>,
) {
    match handler.handle_event(game_context, &event) {
        Ok(_) => {
            out.send(EventFrame::Broadcast {
                state_json: game_context.get_handler_state_json().to_owned(),
                event,
            })
            .await
            .unwrap();
            if let Some(settles) = game_context.extract_settles() {
                out.send(EventFrame::Settle { settles }).await.unwrap();
            }
        }
        Err(e) => warn!("Handle event error: {}", e.to_string()),
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
                    EventFrame::PlayerJoined { new_players } => {
                        info!("Event loop handle player joined");
                        for p in new_players.into_iter() {
                            let event = Event::Join {
                                player_addr: p.addr,
                                balance: p.amount,
                                position: p.position,
                            };
                            handle(&mut handler, &mut game_context, event, &output_tx).await;
                        }
                    }
                    EventFrame::PlayerLeaving { player_addr } => {
                        info!("Event loop handle player leaving");
                        let event = Event::Leave { player_addr };
                        handle(&mut handler, &mut game_context, event, &output_tx).await;
                    }
                    EventFrame::SendEvent { event } => {
                        handle(&mut handler, &mut game_context, event, &output_tx).await;
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
        let (output_tx, output_rx) = mpsc::channel(3);
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
            output_rx: Some(output_rx),
            closed_rx,
            ctx,
        }
    }
}

#[cfg(test)]
mod tests {
    use race_test::*;

    use crate::frame::NewPlayer;

    use super::*;

    #[tokio::test]
    async fn test_player_join() {
        let hdlr = WrappedHandler::load_by_path(
            "../target/wasm32-unknown-unknown/release/race_example_counter.wasm".into(),
        )
        .unwrap();

        let game_account = TestGameAccountBuilder::default()
            .add_servers(1)
            .with_data_vec(vec![0, 0, 0, 42])
            .build();
        let ctx = GameContext::new(&game_account).unwrap();
        let mut event_loop = EventLoop::new(hdlr, ctx);

        let new_player = NewPlayer {
            addr: "Alice".into(),
            position: 0,
            amount: 10000,
        };
        event_loop.start();
        event_loop
            .input_tx
            .send(EventFrame::PlayerJoined {
                new_players: vec![new_player.clone()],
            })
            .await
            .unwrap();
        if event_loop.output_rx.changed().await.is_ok() {
            let ef = event_loop.output_rx.borrow();
            assert_eq!(
                *ef,
                EventFrame::Broadcast {
                    state_json: "{\"value\":42,\"num_of_players\":1,\"num_of_servers\":1}".into(),
                    event: Event::Join {
                        player_addr: new_player.addr,
                        balance: new_player.amount,
                        position: new_player.position,
                    }
                }
            )
        }
    }
}

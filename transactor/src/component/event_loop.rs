use race_core::context::GameContext;
use race_core::error::Error;
use tokio::sync::{mpsc, oneshot, watch};

use crate::component::event_bus::CloseReason;
use crate::component::wrapped_handler::WrappedHandler;
use crate::component::traits::{Attachable, Component, Named};
use race_core::types::{EventFrame, GameAccount};

pub struct EventLoopContext {
    input_rx: mpsc::Receiver<EventFrame>,
    output_tx: watch::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    handler: WrappedHandler,
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
            let handler = ctx.handler;
            let mut fault = false;
            while let Some(event) = ctx.input_rx.recv().await {
                // if let Ok(events) = handler.handle_event(&mut GameContext::default(), event) {
                //     for e in events.into_iter() {
                //         ctx.output_tx.send(e).unwrap();
                //     }
                // } else {
                //     fault = true;
                //     break;
                // }
            }
            if fault {
                ctx.closed_tx.send(CloseReason::Fault).unwrap();
            } else {
                ctx.closed_tx.send(CloseReason::Complete).unwrap();
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
    pub fn new(handler: WrappedHandler, init_state: GameAccount) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
        let (output_tx, output_rx) = watch::channel(EventFrame::Empty);
        let (closed_tx, closed_rx) = oneshot::channel();
        // game_handler.init(init_state).unwrap();

        let ctx = Some(EventLoopContext {
            input_rx,
            output_tx,
            closed_tx,
            handler,
        });
        Self {
            input_tx,
            output_rx,
            closed_rx,
            ctx,
        }
    }
}

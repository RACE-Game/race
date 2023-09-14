use std::time::{Duration, UNIX_EPOCH};

use async_trait::async_trait;
use race_core::context::GameContext;
use race_core::error::Error;
use race_core::event::Event;
use tokio::select;
use tracing::{error, info, warn};

use crate::component::common::{Component, PipelinePorts};
use crate::component::event_bus::CloseReason;
use crate::component::wrapped_handler::WrappedHandler;
use crate::frame::EventFrame;
use race_core::types::{ClientMode, GameAccount};

pub struct EventLoopContext {
    handler: WrappedHandler,
    game_context: GameContext,
    mode: ClientMode,
}

pub trait WrappedGameHandler: Send {
    fn init(&mut self, init_state: GameAccount) -> Result<(), Error>;

    fn handle_event(&mut self, event: EventFrame) -> Result<Vec<EventFrame>, Error>;
}

pub struct EventLoop {}

async fn handle(
    handler: &mut WrappedHandler,
    game_context: &mut GameContext,
    event: Event,
    ports: &PipelinePorts,

    #[allow(unused)] mode: ClientMode,
) -> Option<CloseReason> {
    info!("Handle event: {}", event);

    let access_version = game_context.get_access_version();
    let settle_version = game_context.get_settle_version();

    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            ports
                .send(EventFrame::Broadcast {
                    event,
                    access_version,
                    settle_version,
                    timestamp: game_context.get_timestamp(),
                })
                .await;

            if game_context.is_checkpoint() {
                ports
                    .send(EventFrame::Checkpoint {
                        state: game_context.get_handler_state_raw().to_owned(),
                        access_version: game_context.get_access_version(),
                        settle_version: game_context.get_settle_version(),
                    })
                    .await;
            }

            ports
                .send(EventFrame::ContextUpdated {
                    context: game_context.clone(),
                })
                .await;

            // We do optimistic updates here
            if let Some(effects) = effects {
                info!("Send settlements: {:?}", effects);
                ports
                    .send(EventFrame::Settle {
                        settles: effects.settles,
                        transfers: effects.transfers,
                    })
                    .await;
            }
        }
        Err(e) => {
            warn!("Handle event error: {}", e.to_string());
            // info!("Current context: {:?}", game_context);
            match e {
                Error::WasmExecutionError(_) | Error::WasmMemoryOverflow => {
                    return Some(CloseReason::Fault(e))
                }
                _ => (),
            }
        }
    }
    return None;
}

/// Take the event from clients or the pending dispatched event.
/// Transactor will retrieve events from both dispatching event and
/// ports, while Validator will retrieve events from only ports.
async fn retrieve_event(
    ports: &mut PipelinePorts,
    game_context: &mut GameContext,
    mode: ClientMode,
) -> Option<EventFrame> {
    let timestamp = current_timestamp();
    if mode != ClientMode::Transactor {
        ports.recv().await
    } else if let Some(dispatch) = game_context.get_dispatch() {
        // If already passed
        if dispatch.timeout <= timestamp {
            let event = dispatch.event.clone();
            game_context.cancel_dispatch();
            return Some(EventFrame::SendServerEvent { event });
        }
        let to = tokio::time::sleep(Duration::from_millis(dispatch.timeout - timestamp));
        select! {
            ef = ports.recv() => {
                ef
            }
            _ = to => {
                let event = dispatch.event.clone();
                game_context.cancel_dispatch();
                Some(EventFrame::SendServerEvent { event })
            }
        }
    } else {
        ports.recv().await
    }
}

#[async_trait]
impl Component<PipelinePorts, EventLoopContext> for EventLoop {
    fn name(&self) -> &str {
        "Event Loop"
    }

    async fn run(mut ports: PipelinePorts, ctx: EventLoopContext) -> CloseReason {
        let mut handler = ctx.handler;
        let mut game_context = ctx.game_context;

        // Read games from event bus
        while let Some(event_frame) = retrieve_event(&mut ports, &mut game_context, ctx.mode).await
        {
            // Set timestamp to current time
            // Reset some disposable states.
            game_context.prepare_for_next_event(current_timestamp());

            match event_frame {
                EventFrame::InitState {
                    init_account,
                    state,
                } => {
                    if let Err(e) = game_context
                        .apply_checkpoint(init_account.access_version, init_account.settle_version)
                    {
                        error!("Failed to apply checkpoint: {:?}", e);
                        ports.send(EventFrame::Shutdown).await;
                        return CloseReason::Fault(e);
                    }

                    if let Err(e) = handler.init_state(&mut game_context, &init_account) {
                        error!("Failed to initialize state: {:?}", e);
                        ports.send(EventFrame::Shutdown).await;
                        return CloseReason::Fault(e);
                    }

                    info!(
                        "Initialize game state for {}, access_version = {}, settle_version = {}",
                        init_account.addr, init_account.access_version, init_account.settle_version
                    );

                    info!("Initialize timestamp: {}", current_timestamp());

                    game_context.dispatch_safe(Event::Ready, 0);
                    if let Some(state) = state {
                        game_context.set_handler_state_raw(state);
                    }
                }
                EventFrame::Sync {
                    new_players,
                    new_servers,
                    access_version,
                    transactor_addr,
                } => {
                    let event = Event::Sync {
                        new_players,
                        new_servers,
                        access_version,
                        transactor_addr,
                    };
                    if let Some(close_reason) =
                        handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await
                    {
                        ports.send(EventFrame::Shutdown).await;
                        return close_reason;
                    }
                }
                EventFrame::PlayerLeaving { player_addr } => {
                    let event = Event::Leave { player_addr };
                    if let Some(close_reason) =
                        handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await
                    {
                        ports.send(EventFrame::Shutdown).await;
                        return close_reason;
                    }
                }
                EventFrame::SendEvent { event } => {
                    if let Some(close_reason) =
                        handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await
                    {
                        ports.send(EventFrame::Shutdown).await;
                        return close_reason;
                    }
                }
                EventFrame::SendServerEvent { event } => {
                    // Handle the shutdown event from game logic
                    if matches!(event, Event::Shutdown) {
                        ports.send(EventFrame::Shutdown).await;
                        return CloseReason::Complete;
                    } else {
                        if let Some(close_reason) =
                            handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await
                        {
                            ports.send(EventFrame::Shutdown).await;
                            return close_reason;
                        }
                    }
                }
                EventFrame::Shutdown => {
                    warn!("Shutdown event loop");
                    return CloseReason::Complete;
                }
                _ => (),
            }
        }

        return CloseReason::Complete;
    }
}

impl EventLoop {
    pub fn init(
        handler: WrappedHandler,
        game_context: GameContext,
        mode: ClientMode,
    ) -> (Self, EventLoopContext) {
        (
            Self {},
            EventLoopContext {
                handler,
                game_context,
                mode,
            },
        )
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

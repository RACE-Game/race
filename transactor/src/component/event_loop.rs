use sha256::digest;
use std::time::{Duration, UNIX_EPOCH};

use async_trait::async_trait;
use race_api::error::Error;
use race_api::event::Event;
use race_core::context::GameContext;
use tokio::select;
use tracing::{error, info, warn};

use crate::component::common::{Component, PipelinePorts};
use crate::component::event_bus::CloseReason;
use crate::component::wrapped_handler::WrappedHandler;
use crate::frame::EventFrame;
use race_core::types::{ClientMode, GameAccount, GameMode, GamePlayer, SubGameSpec};

use super::ComponentEnv;

fn log_execution_context(ctx: &GameContext, evt: &Event) {
    info!("Execution context");
    info!("===== State =====");
    info!("{:?}", ctx.get_handler_state_raw());
    info!("===== Event =====");
    info!("{:?}", evt);
    info!("=================");
}

pub struct EventLoopContext {
    handler: WrappedHandler,
    game_context: GameContext,
    client_mode: ClientMode,
    game_mode: GameMode,
}

pub trait WrappedGameHandler: Send {
    fn init(&mut self, init_state: GameAccount) -> Result<(), Error>;

    fn handle_event(&mut self, event: EventFrame) -> Result<Vec<EventFrame>, Error>;
}

pub struct EventLoop {}

async fn handle_event(
    handler: &mut WrappedHandler,
    game_context: &mut GameContext,
    event: Event,
    ports: &PipelinePorts,
    client_mode: ClientMode,
    game_mode: GameMode,
    env: &ComponentEnv,
) -> Option<CloseReason> {
    info!("{} Handle event: {}, timestamp: {}", env.log_prefix, event, game_context.get_timestamp());

    let access_version = game_context.access_version();
    let settle_version = game_context.settle_version();

    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            let state = game_context.get_handler_state_raw();
            let state_sha = digest(state);
            let timestamp = game_context.get_timestamp();
            // info!("{} Game state SHA: {}", env.log_prefix, state_sha);

            // Broacast the event to clients
            if client_mode == ClientMode::Transactor {
                ports
                    .send(EventFrame::Broadcast {
                        event,
                        access_version,
                        settle_version,
                        timestamp,
                        state_sha: state_sha.clone(),
                    })
                    .await;
            }

            // Update the local client
            ports
                .send(EventFrame::ContextUpdated {
                    context: Box::new(game_context.clone()),
                })
                .await;

            // Start game
            if client_mode == ClientMode::Transactor && effects.start_game {
                ports
                    .send(EventFrame::GameStart {
                        access_version: game_context.access_version(),
                    })
                    .await;
            }

            // Send the settlement when there's one
            if let Some(checkpoint) = effects.checkpoint {
                info!(
                    "{} Create checkpoint, settle_version: {}",
                    env.log_prefix,
                    game_context.settle_version(),
                );
                ports
                    .send(EventFrame::Checkpoint {
                        access_version: game_context.access_version(),
                        settle_version: game_context.settle_version(),
                        previous_settle_version: settle_version,
                        checkpoint,
                        settles: effects.settles,
                        transfers: effects.transfers,
                    })
                    .await;
            }

            // Launch sub games
            if game_mode == GameMode::Main {
                for launch_sub_game in effects.launch_sub_games {
                    info!("{} Launch sub game: {}", env.log_prefix, launch_sub_game.id);
                    let cp = game_context.checkpoint_mut();
                    cp.maybe_init_data(launch_sub_game.id, &launch_sub_game.init_account.data);
                    let settle_version = cp.get_version(launch_sub_game.id);

                    let ef = EventFrame::LaunchSubGame {
                        spec: Box::new(SubGameSpec {
                            game_addr: game_context.game_addr().to_owned(),
                            game_id: launch_sub_game.id,
                            bundle_addr: launch_sub_game.bundle_addr,
                            nodes: game_context.get_nodes().into(),
                            access_version: game_context.access_version(),
                            settle_version,
                            init_account: launch_sub_game.init_account,
                            checkpoint_state: launch_sub_game.checkpoint_state,
                        }),
                    };
                    ports.send(ef).await;
                }
            }

            // Emit bridge events
            if client_mode == ClientMode::Transactor {
                for be in effects.bridge_events {
                    info!("{} Send bridge event, dest: {}", env.log_prefix, be.dest);
                    let ef = EventFrame::SendBridgeEvent {
                        from: game_context.game_id(),
                        dest: be.dest,
                        event: Event::Bridge {
                            dest: be.dest,
                            raw: be.raw,
                            join_players: be.join_players,
                        },
                        access_version: game_context.access_version(),
                        settle_version: game_context.settle_version(),
                        checkpoint: game_context.checkpoint().data(game_context.game_id()),
                    };
                    ports.send(ef).await;
                }
            }
        }
        Err(e) => {
            warn!("{} Handle event error: {}", env.log_prefix, e.to_string());
            log_execution_context(game_context, &event);
            match e {
                Error::WasmExecutionError(_) | Error::WasmMemoryOverflow => {
                    return Some(CloseReason::Fault(e))
                }
                _ => (),
            }
        }
    }
    None
}

/// Take the event from clients or the pending dispatched event.
/// Transactor will retrieve events from both dispatching event and
/// ports, while Validator will retrieve events from only ports.
async fn read_event(
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
    fn name() -> &'static str {
        "Event Loop"
    }

    async fn run(
        mut ports: PipelinePorts,
        ctx: EventLoopContext,
        env: ComponentEnv,
    ) -> CloseReason {
        let mut handler = ctx.handler;
        let mut game_context = ctx.game_context;

        // Read games from event bus
        while let Some(event_frame) =
            read_event(&mut ports, &mut game_context, ctx.client_mode).await
        {
            game_context.prepare_for_next_event(current_timestamp());

            match event_frame {
                EventFrame::InitState {
                    init_account,
                    access_version,
                    settle_version,
                    checkpoint_state,
                } => {
                    if let Err(e) = game_context.apply_checkpoint(access_version, settle_version) {
                        error!("{} Failed to apply checkpoint: {:?}, context settle version: {}, init account settle version: {}", env.log_prefix, e,
                            game_context.settle_version(), settle_version);
                        ports.send(EventFrame::Shutdown).await;
                        return CloseReason::Fault(e);
                    }

                    if let Err(e) = handler.init_state(&mut game_context, &init_account, checkpoint_state) {
                        error!("{} Failed to initialize state: {:?}", env.log_prefix, e);
                        ports.send(EventFrame::Shutdown).await;
                        return CloseReason::Fault(e);
                    }

                    let state_sha = digest(game_context.get_handler_state_raw());
                    info!(
                        "{} Initialize game state, SHA: {}",
                        env.log_prefix, state_sha
                    );
                    info!(
                        "{} Init Account: {:?}", env.log_prefix, init_account
                    );

                    game_context.dispatch_safe(Event::Ready, 0);
                }

                EventFrame::GameStart { .. } => {
                    if ctx.client_mode == ClientMode::Transactor {
                        let event = Event::GameStart;
                        if let Some(close_reason) = handle_event(
                            &mut handler,
                            &mut game_context,
                            event,
                            &ports,
                            ctx.client_mode,
                            ctx.game_mode,
                            &env,
                        )
                            .await
                        {
                            ports.send(EventFrame::Shutdown).await;
                            return close_reason;
                        }
                    }
                }

                EventFrame::Sync {
                    new_players,
                    new_servers,
                    access_version,
                    transactor_addr,
                } => {
                    info!(
                        "{} handle Sync, access_version: {:?}",
                        env.log_prefix, access_version
                    );
                    game_context.set_access_version(access_version);

                    // Add servers to context
                    for server in new_servers.iter() {
                        let mode = if server.addr.eq(&transactor_addr) {
                            ClientMode::Transactor
                        } else {
                            ClientMode::Validator
                        };
                        game_context.add_node(server.addr.clone(), server.access_version, mode);
                        info!(
                            "{} Game context add server: {:?}",
                            env.log_prefix, server.addr
                        );
                    }

                    let mut new_players_1: Vec<GamePlayer> = Vec::with_capacity(new_players.len());
                    for p in new_players.iter() {
                        new_players_1.push(p.clone().into());
                        game_context.add_node(p.addr.clone(), p.access_version, ClientMode::Player);
                    }

                    // We only generate join event in Transactor & Main mode.
                    if ctx.client_mode == ClientMode::Transactor
                        && ctx.game_mode == GameMode::Main
                        && !new_players.is_empty()
                    {
                        let event = Event::Join {
                            players: new_players_1,
                        };
                        if let Some(close_reason) = handle_event(
                            &mut handler,
                            &mut game_context,
                            event,
                            &ports,
                            ctx.client_mode,
                            ctx.game_mode,
                            &env,
                        )
                            .await
                        {
                            ports.send(EventFrame::Shutdown).await;
                            return close_reason;
                        }
                    }
                }
                EventFrame::PlayerLeaving { player_addr } => {
                    info!("Current allow_exit = {}", game_context.is_allow_exit());
                    if let Ok(player_id) = game_context.addr_to_id(&player_addr) {
                        let event = Event::Leave { player_id };
                        if let Some(close_reason) = handle_event(
                            &mut handler,
                            &mut game_context,
                            event,
                            &ports,
                            ctx.client_mode,
                            ctx.game_mode,
                            &env,
                        )
                            .await
                        {
                            ports.send(EventFrame::Shutdown).await;
                            return close_reason;
                        }
                    } else {
                        error!(
                            "{} Ignore PlayerLeaving, due to can not map the address to id",
                            env.log_prefix
                        );
                    }
                }
                EventFrame::RecvBridgeEvent {
                    event,
                    dest,
                    from,
                    checkpoint,
                    ..
                } => {
                    if game_context.game_id() == 0 && dest == 0 && from != 0 {
                        if let Err(e) = game_context.checkpoint_mut().set_data(
                            from,
                            checkpoint,
                        ) {
                            error!("Failed to update checkpoint at {} due to {:?}", from, e);
                        }
                    }

                    if let Some(close_reason) = handle_event(
                        &mut handler,
                        &mut game_context,
                        event,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        &env,
                    )
                        .await
                    {
                        ports.send(EventFrame::Shutdown).await;
                        return close_reason;
                    }
                }
                EventFrame::SendEvent { event } => {
                    if let Some(close_reason) = handle_event(
                        &mut handler,
                        &mut game_context,
                        event,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        &env,
                    )
                        .await
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
                    } else if let Some(close_reason) = handle_event(
                        &mut handler,
                        &mut game_context,
                        event,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        &env,
                    )
                        .await
                    {
                        ports.send(EventFrame::Shutdown).await;
                        return close_reason;
                    }
                }
                EventFrame::Shutdown => {
                    warn!("{} Shutdown event loop", env.log_prefix);
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
        client_mode: ClientMode,
        game_mode: GameMode,
    ) -> (Self, EventLoopContext) {
        (
            Self {},
            EventLoopContext {
                handler,
                game_context,
                client_mode,
                game_mode,
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

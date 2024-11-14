use race_api::types::GameDeposit;

use async_trait::async_trait;
use race_api::error::Error;
use race_api::event::Event;
use race_core::context::GameContext;
use tracing::{error, info, warn};

use crate::component::common::{Component, PipelinePorts};
use crate::component::event_bus::CloseReason;
use crate::component::wrapped_handler::WrappedHandler;
use crate::frame::EventFrame;
use crate::utils::current_timestamp;
use race_core::types::{ClientMode, GameAccount, GameMode, GamePlayer};

use super::ComponentEnv;

mod misc;
mod event_handler;

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
            misc::read_event(&mut ports, &mut game_context, ctx.client_mode).await
        {
            match event_frame {
                EventFrame::InitState {
                    init_account,
                    access_version,
                    settle_version,
                    ..
                } => {
                    if let Some(close_reason) = event_handler::init_state(
                        init_account,
                        access_version,
                        settle_version,
                        &mut handler,
                        &mut game_context,
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

                EventFrame::GameStart { .. } => {
                    let timestamp = current_timestamp();
                    if ctx.client_mode == ClientMode::Transactor {
                        let event = Event::GameStart;
                        if let Some(close_reason) = event_handler::handle_event(
                            &mut handler,
                            &mut game_context,
                            event,
                            &ports,
                            ctx.client_mode,
                            ctx.game_mode,
                            timestamp,
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
                    new_deposits,
                    access_version,
                    transactor_addr,
                } => {
                    let timestamp = current_timestamp();

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

                    let mut players: Vec<GamePlayer> = Vec::with_capacity(new_players.len());
                    let mut deposits: Vec<GameDeposit> = Vec::with_capacity(new_deposits.len());

                    for player in new_players.iter() {
                        players.push(GamePlayer::new(player.access_version, player.position));
                        game_context.add_node(player.addr.clone(), player.access_version, ClientMode::Player);
                    }

                    for deposit in new_deposits.iter() {
                        if let Ok(id) = game_context.addr_to_id(&deposit.addr) {
                            deposits.push(GameDeposit::new(id, deposit.amount));
                        } else {
                            error!("A deposit cannot be resolved, addr: {}", deposit.addr);
                        }
                    }

                    // We only generate join event in Transactor & Main mode.
                    if ctx.client_mode == ClientMode::Transactor && ctx.game_mode == GameMode::Main {
                        // Send new players
                        if !players.is_empty() {
                            let event = Event::Join {
                                players
                            };
                            if let Some(close_reason) = event_handler::handle_event(
                                &mut handler,
                                &mut game_context,
                                event,
                                &ports,
                                ctx.client_mode,
                                ctx.game_mode,
                                timestamp,
                                &env,
                            ).await {
                                ports.send(EventFrame::Shutdown).await;
                                return close_reason;
                            }
                        }
                        // Send new deposits
                        if !deposits.is_empty() {
                            let event = Event::Deposit {
                                deposits
                            };
                            if let Some(close_reason) = event_handler::handle_event(
                                &mut handler,
                                &mut game_context,
                                event,
                                &ports,
                                ctx.client_mode,
                                ctx.game_mode,
                                timestamp,
                                &env,
                            ).await {
                                ports.send(EventFrame::Shutdown).await;
                                return close_reason;
                            }
                        }
                    }
                }
                EventFrame::PlayerLeaving { player_addr } => {
                    let timestamp = current_timestamp();
                    if let Ok(player_id) = game_context.addr_to_id(&player_addr) {
                        let event = Event::Leave { player_id };
                        if let Some(close_reason) = event_handler::handle_event(
                            &mut handler,
                            &mut game_context,
                            event,
                            &ports,
                            ctx.client_mode,
                            ctx.game_mode,
                            timestamp,
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
                    checkpoint_state,
                    settle_version,
                    ..
                } => {
                    // In the case of parent, update the child game's
                    // checkpoint value.

                    let timestamp = current_timestamp();

                    if game_context.game_id() == 0 && dest == 0 && from != 0 && settle_version > 0 {
                        info!("Update checkpoint for child game: {}", from);
                        game_context.checkpoint_mut().set_data(from, checkpoint_state)
                    }

                    if let Some(close_reason) = event_handler::handle_event(
                        &mut handler,
                        &mut game_context,
                        event,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        timestamp,
                        &env,
                    )
                        .await
                    {
                        ports.send(EventFrame::Shutdown).await;
                        return close_reason;
                    }
                }
                EventFrame::SendEvent { event, timestamp } => {
                    if let Some(close_reason) = event_handler::handle_event(
                        &mut handler,
                        &mut game_context,
                        event,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        timestamp,
                        &env,
                    )
                        .await
                    {
                        ports.send(EventFrame::Shutdown).await;
                        return close_reason;
                    }
                }
                EventFrame::SendServerEvent { event, timestamp } => {
                    // Handle the shutdown event from game logic
                    if matches!(event, Event::Shutdown) {
                        ports.send(EventFrame::Shutdown).await;
                        return CloseReason::Complete;
                    } else if let Some(close_reason) = event_handler::handle_event(
                        &mut handler,
                        &mut game_context,
                        event,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        timestamp,
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

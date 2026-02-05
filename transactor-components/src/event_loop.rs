use std::sync::Arc;

use race_api::types::GameDeposit;
use async_trait::async_trait;
use race_api::event::Event;
use race_core::context::GameContext;
use race_core::game_spec::GameSpec;
use race_core::encryptor::EncryptorT;
use tracing::{error, info, warn};
use race_handler::HandlerManager;
use race_core::transport::TransportT;

use crate::common::{Component, PipelinePorts};
use crate::event_bus::CloseReason;
use race_transactor_frames::EventFrame;

use crate::utils::current_timestamp;
use race_core::types::{ClientMode, GameMode, GamePlayer};

use super::ComponentEnv;

mod event_handler;
mod misc;

pub struct EventLoopContext {
    game_spec: GameSpec,
    client_mode: ClientMode,
    game_mode: GameMode,
    encryptor: Arc<dyn EncryptorT>,
    transport: Arc<dyn TransportT>,
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
        // Create an empty game context, replace it later with the real one.
        let mut game_context = GameContext::default();
        let game_spec = ctx.game_spec;
        let encryptor = ctx.encryptor;

        let mut handler_manager = HandlerManager::new(ctx.transport);

        let mut handler = match handler_manager.get_handler(&game_spec.bundle_addr).await {
            Ok(handler) => handler,
            Err(e) => {
                return CloseReason::Fault(e)
            }
        };

        // Read games from event bus
        while let Some(event_frame) =
            misc::read_event(&mut ports, &mut game_context, ctx.client_mode).await
        {

            match event_frame {
                // The first initialization, this runs only once for each game
                EventFrame::InitState {
                    access_version,
                    settle_version,
                    nodes,
                    init_account,
                } => {
                    match event_handler::init_state(
                        access_version,
                        settle_version,
                        nodes,
                        &mut *handler,
                        &game_spec,
                        init_account,
                        &ports,
                        ctx.game_mode,
                        &env,
                    ).await {
                        Ok(ctx) => {
                            game_context = ctx;
                        }
                        Err(e) => {
                            return CloseReason::Fault(e);
                        }
                    }
                }

                // The initialization for a game that has a checkpoint.
                // It's one of these three cases:
                // 1. The transactor is resuming a game
                // 2. The validator is initializing a game
                // 3. The sub game is initializing (sub game is always started with a checkpoint)
                EventFrame::RecoverCheckpointWithCredentials {
                    checkpoint,
                } => {
                    match event_handler::recover_from_checkpoint(
                        &checkpoint,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        &env,
                    ).await {
                        Ok(ctx) => {
                            game_context = ctx;
                        }
                        Err(e) => {
                            return CloseReason::Fault(e);
                        }
                    }

                }

                EventFrame::SubSync {
                    access_version,
                    new_players,
                    new_servers,
                    transactor_addr,
                } => {

                    info!(
                        "{} handle SubSync, access_version: {}",
                        env.log_prefix, access_version
                    );
                    game_context.set_access_version(access_version);

                    for server in new_servers.iter() {
                        let mode = if server.addr.eq(&transactor_addr) {
                            ClientMode::Transactor
                        } else {
                            ClientMode::Validator
                        };
                        info!(
                            "{} Game context add server: {}, mode: {:?}",
                            env.log_prefix, server.addr, mode
                        );
                        game_context.add_node(server.addr.clone(), server.access_version, mode);
                    }
                    for player in new_players.iter() {
                        info!(
                            "{} Game context add player: {}",
                            env.log_prefix, player.addr
                        );
                        game_context.add_node(
                            player.addr.clone(),
                            player.access_version,
                            ClientMode::Player,
                        );
                    }
                }

                EventFrame::SyncWithCredentials {
                    new_players,
                    new_servers,
                    new_deposits,
                    access_version,
                    transactor_addr,
                } => {
                    let timestamp = current_timestamp();

                    info!(
                        "{} handle Sync, access_version: {}",
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
                            "{} Game context add server: {}",
                            env.log_prefix, server.addr
                        );
                    }

                    let mut players: Vec<GamePlayer> = Vec::with_capacity(new_players.len());
                    let mut deposits: Vec<GameDeposit> = Vec::with_capacity(new_deposits.len());

                    for player in new_players.iter() {
                        players.push(GamePlayer::new(player.access_version));
                        game_context.add_node(
                            player.addr.clone(),
                            player.access_version,
                            ClientMode::Player,
                        );
                    }

                    for deposit in new_deposits.iter() {
                        if let Ok(id) = game_context.addr_to_id(&deposit.addr) {
                            deposits.push(GameDeposit::new(
                                id,
                                deposit.amount,
                                deposit.access_version,
                            ));
                        } else {
                            warn!(
                                "A deposit cannot be resolved, addr: {}, access_version: {}",
                                deposit.addr, deposit.access_version
                            );
                        }
                    }

                    // We only generate join event in Transactor & Main mode.
                    if ctx.client_mode == ClientMode::Transactor && ctx.game_mode == GameMode::Main
                    {
                        // Send new players
                        if !players.is_empty() {
                            let event = Event::Join { players };
                            if let Some(close_reason) = event_handler::handle_event(
                                &mut *handler,
                                &mut handler_manager,
                                &mut game_context,
                                event,
                                &*encryptor,
                                &ports,
                                ctx.client_mode,
                                ctx.game_mode,
                                timestamp,
                                &env,
                            )
                            .await
                            {
                                return close_reason;
                            }
                        }
                        // Send new deposits
                        if !deposits.is_empty() {
                            let event = Event::Deposit { deposits };
                            if let Some(close_reason) = event_handler::handle_event(
                                &mut *handler,
                                &mut handler_manager,
                                &mut game_context,
                                event,
                                &*encryptor,
                                &ports,
                                ctx.client_mode,
                                ctx.game_mode,
                                timestamp,
                                &env,
                            )
                            .await
                            {
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
                            &mut *handler,
                            &mut handler_manager,
                            &mut game_context,
                            event,
                            &*encryptor,
                            &ports,
                            ctx.client_mode,
                            ctx.game_mode,
                            timestamp,
                            &env,
                        )
                        .await
                        {
                            return close_reason;
                        }
                    } else {
                        error!(
                            "{} Ignore PlayerLeaving, due to can not map the address to id",
                            env.log_prefix
                        );
                    }
                }

                EventFrame::SubGameReady {
                    versioned_data,
                    game_id,
                    init_data,
                    max_players,
                } => {
                    if ctx.game_mode == GameMode::Main && ctx.client_mode == ClientMode::Transactor
                    {
                        info!("SubGameReady: Update checkpoint for sub game: {}", game_id);
                        if let Err(e) =
                            game_context.init_sub_game_data(versioned_data)
                        {
                            error!(
                                "{} Failed in handling new sub game's versioned data: {:?}",
                                env.log_prefix, e
                            );
                            ports.send(EventFrame::Shutdown).await;
                            return CloseReason::Fault(e);
                        }
                        let timestamp = current_timestamp();
                        let event = Event::SubGameReady {
                            game_id,
                            max_players,
                            init_data,
                        };
                        if let Some(close_reason) = event_handler::handle_event(
                            &mut *handler,
                            &mut handler_manager,
                            &mut game_context,
                            event,
                            &*encryptor,
                            &ports,
                            ctx.client_mode,
                            ctx.game_mode,
                            timestamp,
                            &env,
                        )
                        .await
                        {
                            return close_reason;
                        }
                    }
                }

                EventFrame::SubGameShutdown {
                    game_id,
                    versioned_data,
                } => {
                    if ctx.game_mode == GameMode::Main
                        && ctx.client_mode == ClientMode::Transactor
                        && game_context.game_id() == 0
                    {
                        info!(
                            "SubGameShutdown: Update checkpoint for sub game: {}",
                            game_id
                        );
                        if let Err(e) =
                            game_context.update_sub_game_data(versioned_data)
                        {
                            error!(
                                "{} SubGameShutdown: Failed in handling new sub game's versioned data: {:?}",
                                env.log_prefix, e
                            );
                            ports.send(EventFrame::Shutdown).await;
                            return CloseReason::Fault(e);
                        }
                    }
                }

                EventFrame::RecvBridgeEvent {
                    event,
                    dest,
                    from,
                    versioned_data,
                    ..
                } => {
                    // In the case of parent, update the child game' checkpoint value.
                    let timestamp = current_timestamp();
                    let settle_version = versioned_data.versions.settle_version;

                    if game_context.game_id() == 0 && dest == 0 && from != 0 && settle_version > 0 {
                        info!("BridgeEvent: Update checkpoint for sub game: {}", from);
                        if let Err(e) =
                            game_context.update_sub_game_data(versioned_data)
                        {
                            error!(
                                "{} Failed in handling new sub game's versioned data: {:?}",
                                env.log_prefix, e
                            );
                            ports.send(EventFrame::Shutdown).await;
                            return CloseReason::Fault(e);
                        }
                    }

                    if let Some(close_reason) = event_handler::handle_event(
                        &mut *handler,
                        &mut handler_manager,
                        &mut game_context,
                        event,
                        &*encryptor,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        timestamp,
                        &env,
                    )
                    .await
                    {
                        return close_reason;
                    }
                }
                EventFrame::SendEvent { event, timestamp } => {
                    if let Some(close_reason) = event_handler::handle_event(
                        &mut *handler,
                        &mut handler_manager,
                        &mut game_context,
                        event,
                        &*encryptor,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        timestamp,
                        &env,
                    )
                    .await
                    {
                        return close_reason;
                    }
                }
                EventFrame::SendServerEvent { event, timestamp } => {
                    // Handle the shutdown event from game logic
                    if matches!(event, Event::Shutdown) {
                        return CloseReason::Complete;
                    }

                    if let Some(close_reason) = event_handler::handle_event(
                        &mut *handler,
                        &mut handler_manager,
                        &mut game_context,
                        event,
                        &*encryptor,
                        &ports,
                        ctx.client_mode,
                        ctx.game_mode,
                        timestamp,
                        &env,
                    )
                    .await
                    {
                        return close_reason;
                    }
                }
                EventFrame::Shutdown => {
                    info!("{} Stopped", env.log_prefix);
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
        game_spec: GameSpec,
        encryptor: Arc<dyn EncryptorT>,
        transport: Arc<dyn TransportT>,
        client_mode: ClientMode,
        game_mode: GameMode,
    ) -> (Self, EventLoopContext) {
        (
            Self {},
            EventLoopContext {
                game_spec,
                client_mode,
                game_mode,
                encryptor,
                transport,
            },
        )
    }
}

#[cfg(test)]
mod tests {

    use borsh::{BorshDeserialize, BorshSerialize};
    use race_api::{event::BridgeEvent, prelude::InitAccount};
    use race_core::error::Result;
    use race_core::{checkpoint::VersionedData, context::EventEffects};

    use super::*;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct EmptyBridgeEvent {}

    impl BridgeEvent for EmptyBridgeEvent {}

    struct TestHandlerForBridgeEvent {}

    impl HandlerT for TestHandlerForBridgeEvent {
        fn init_state(
            &mut self,
            _context: &mut GameContext,
            _init_account: &InitAccount,
        ) -> Result<EventEffects> {
            Ok(EventEffects::default())
        }

        fn handle_event(
            &mut self,
            context: &mut GameContext,
            _event: &Event,
        ) -> Result<EventEffects> {
            let mut ef = context.derive_effect(false);
            ef.checkpoint();
            ef.bridge_event(0, EmptyBridgeEvent {})?;
            let ee = context.apply_effect(ef);
            ee
        }
    }

    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_event_loop_receive_bridge_event() {
        let handler = TestHandlerForBridgeEvent {};
        let game_context = GameContext::default();

        let (event_loop, event_loop_ctx) = EventLoop::init(
            Box::new(handler),
            game_context,
            ClientMode::Transactor,
            GameMode::Main,
        );
        let mut event_loop_handle = event_loop.start("fake addresses", event_loop_ctx);
        let mut vd1 = VersionedData::default();
        vd1.id = 1;
        event_loop_handle
            .send_unchecked(EventFrame::RecvBridgeEvent {
                from: 1,
                dest: 0,
                event: Event::Bridge {
                    dest_game_id: 0,
                    from_game_id: 1,
                    raw: vec![],
                },
                versioned_data: vd1,
            })
            .await;
        println!("Sent!");
        let recv = event_loop_handle.recv_unchecked().await;
        println!("{recv:?}");
        assert!(false);
    }
}

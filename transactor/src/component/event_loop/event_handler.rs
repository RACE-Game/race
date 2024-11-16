use super::misc::log_execution_context;
use race_api::{
    effect::{EmitBridgeEvent, SubGame},
    error::Error,
    event::Event,
    prelude::InitAccount,
    types::{EntryLock, Settle, Transfer},
};
use race_core::{
    checkpoint::Checkpoint, context::{ContextVersions, EventEffects, GameContext}, types::{ClientMode, GameMode, SubGameSpec}
};

use crate::{
    component::{common::PipelinePorts, CloseReason, ComponentEnv, WrappedHandler},
    frame::EventFrame,
};
use tracing::{error, info, warn};

async fn broadcast_event(
    event: Event,
    original_versions: ContextVersions,
    game_context: &GameContext,
    ports: &PipelinePorts,
) {
    ports
        .send(EventFrame::Broadcast {
            event,
            access_version: original_versions.access_version,
            settle_version: original_versions.settle_version,
            timestamp: game_context.get_timestamp(),
            state_sha: game_context.state_sha(),
        })
        .await;
}

async fn update_local_client(game_context: &GameContext, ports: &PipelinePorts) {
    ports
        .send(EventFrame::ContextUpdated {
            context: Box::new(game_context.clone()),
        })
        .await;
}

async fn send_start_game(game_context: &GameContext, ports: &PipelinePorts) {
    ports
        .send(EventFrame::GameStart {
            access_version: game_context.access_version(),
        })
        .await;
}

async fn send_bridge_event(
    bridge_events: Vec<EmitBridgeEvent>,
    game_context: &GameContext,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    for be in bridge_events {
        info!(
            "{} Send bridge event, dest: {}",
            env.log_prefix, be.dest
        );
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
            checkpoint_state: game_context.get_handler_state_raw().to_owned(),
        };
        ports.send(ef).await;
    }
}

async fn send_settlement(
    checkpoint: Checkpoint,
    transfers: Vec<Transfer>,
    settles: Vec<Settle>,
    entry_lock: Option<EntryLock>,
    original_versions: ContextVersions,
    game_context: &GameContext,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    let checkpoint_size = checkpoint.get_data(game_context.game_id()).map(|d| d.len());
    info!(
        "{} Create checkpoint, settle_version: {}, size: {:?}",
        env.log_prefix,
        game_context.settle_version(),
        checkpoint_size,
    );

    ports
        .send(EventFrame::Checkpoint {
            access_version: game_context.access_version(),
            settle_version: game_context.settle_version(),
            previous_settle_version: original_versions.settle_version,
            checkpoint: checkpoint.clone(),
            settles,
            transfers,
            state_sha: game_context.state_sha(),
            entry_lock
        })
        .await;
}

async fn launch_sub_game(
    sub_games: Vec<SubGame>,
    game_context: &mut GameContext,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    for sub_game in sub_games {
        info!("{} Launch sub game: {}", env.log_prefix, sub_game.id);
        let cp = game_context.checkpoint_mut();
        let SubGame {
            bundle_addr,
            id,
            init_account,
        } = sub_game;
        // Use the existing checkpoint when possible.
        let checkpoint = cp.clone();
        let settle_version = cp.get_version(id);

        let ef = EventFrame::LaunchSubGame {
            spec: Box::new(SubGameSpec {
                game_addr: game_context.game_addr().to_owned(),
                game_id: id,
                bundle_addr,
                nodes: game_context.get_nodes().into(),
                access_version: game_context.access_version(),
                settle_version,
                init_account,
            }),
            checkpoint,
        };
        ports.send(ef).await;
    }
}

pub async fn init_state(
    init_account: InitAccount,
    access_version: u64,
    settle_version: u64,
    handler: &mut WrappedHandler,
    mut game_context: &mut GameContext,
    ports: &PipelinePorts,
    _client_mode: ClientMode,
    game_mode: GameMode,
    env: &ComponentEnv,
) -> Option<CloseReason> {

    let original_versions = game_context.versions();

    let effects = match handler.init_state(&mut game_context, &init_account) {
        Ok(effects) => effects,
        Err(e) => {
            error!("{} Failed to initialize state: {:?}", env.log_prefix, e);
            error!("{} Init Account: {:?}", env.log_prefix, init_account);
            ports.send(EventFrame::Shutdown).await;
            return Some(CloseReason::Fault(e));
        }
    };

    let EventEffects { checkpoint, launch_sub_games, .. } = effects;

    info!(
        "{} Initialize game state, access_version: {}, settle_version: {}, SHA: {}",
        env.log_prefix, access_version, settle_version, game_context.state_sha()
    );

    if let Some(checkpoint) = checkpoint {
        send_settlement(checkpoint, vec![], vec![], None, original_versions, &game_context, ports, env).await;
    }

    game_context.dispatch_safe(Event::Ready, 0);

    if game_mode == GameMode::Main {
        launch_sub_game(launch_sub_games, game_context, ports, env).await;
    }
    return None;
}

pub async fn handle_event(
    handler: &mut WrappedHandler,
    game_context: &mut GameContext,
    event: Event,
    ports: &PipelinePorts,
    client_mode: ClientMode,
    game_mode: GameMode,
    timestamp: u64,
    env: &ComponentEnv,
) -> Option<CloseReason> {
    info!(
        "{} Handle event: {}, timestamp: {}",
        env.log_prefix, event, timestamp
    );

    game_context.set_timestamp(timestamp);
    let original_versions = game_context.versions();

    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            let EventEffects { settles, transfers, checkpoint, launch_sub_games, bridge_events, start_game, entry_lock } = effects;

            // Broacast the event to clients
            if client_mode == ClientMode::Transactor {
                broadcast_event(event, original_versions, &game_context, ports).await;
            }

            // Update the local client
            update_local_client(&game_context, ports).await;

            // Start game
            if client_mode == ClientMode::Transactor && start_game {
                send_start_game(&game_context, ports).await;
            }

            // Send the settlement when there's one
            // This event will be sent no matter what client mode we are running at
            if let Some(checkpoint) = checkpoint {
                send_settlement(checkpoint, transfers, settles, entry_lock, original_versions, &game_context, ports, env).await;
            }

            // Launch sub games
            if game_mode == GameMode::Main {
                launch_sub_game(launch_sub_games, game_context, ports, env).await;
            }

            // Emit bridge events
            if client_mode == ClientMode::Transactor {
                send_bridge_event(bridge_events, &game_context, ports, env).await;
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

use super::misc::log_execution_context;
use race_api::{
    effect::{EmitBridgeEvent, Log, SubGame},
    event::Event,
    types::{Award, EntryLock, Settle, Transfer},
};
use race_core::{
    context::{EventEffects, GameContext, SubGameInit, SubGameInitSource, Versions},
    error::Error,
    types::{ClientMode, GameMode, GameSpec},
};

use crate::{
    component::{common::PipelinePorts, CloseReason, ComponentEnv, WrappedHandler},
    frame::EventFrame,
};
use tracing::{debug, error, info, warn};

fn print_logs(logs: &[Log], env: &ComponentEnv) {
    logs.iter().for_each(|log| match log.level {
        race_api::effect::LogLevel::Debug => {
            debug!("[{}|Game] {}", env.addr_shorthand, log.message)
        }
        race_api::effect::LogLevel::Info => info!("[{}|Game] {}", env.addr_shorthand, log.message),
        race_api::effect::LogLevel::Warn => warn!("[{}|Game] {}", env.addr_shorthand, log.message),
        race_api::effect::LogLevel::Error => {
            error!("[{}|Game] {}", env.addr_shorthand, log.message)
        }
    })
}

async fn broadcast_event(
    event: Event,
    game_context: &GameContext,
    ports: &PipelinePorts,
) {
    ports
        .send(EventFrame::Broadcast {
            event,
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
        info!("{} Send bridge event, dest: {}", env.log_prefix, be.dest);
        let checkpoint_state = game_context
            .checkpoint()
            .get_versioned_data(game_context.game_id());
        let Some(checkpoint_state) = checkpoint_state else {
            error!(
                "{} Checkpoint for current game not found when preparing bridge event",
                env.log_prefix
            );
            continue;
        };
        let ef = EventFrame::SendBridgeEvent {
            from: game_context.game_id(),
            dest: be.dest,
            event: Event::Bridge {
                dest_game_id: be.dest,
                from_game_id: game_context.game_id(),
                raw: be.raw,
            },
            access_version: game_context.access_version(),
            settle_version: game_context.settle_version(),
            checkpoint_state: checkpoint_state.clone(),
        };
        ports.send(ef).await;
    }
}

async fn send_reject_deposits(
    reject_deposits: Vec<u64>,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {

    info!("{} Send reject deposits, {:?}", env.log_prefix, reject_deposits);

    let ef = EventFrame::RejectDeposits { reject_deposits };

    ports.send(ef).await;
}

async fn send_settlement(
    transfer: Option<Transfer>,
    settles: Vec<Settle>,
    awards: Vec<Award>,
    entry_lock: Option<EntryLock>,
    original_versions: Versions,
    game_context: &mut GameContext,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    let checkpoint = game_context.checkpoint().clone();
    let checkpoint_size = checkpoint.get_data(game_context.game_id()).map(|d| d.len());
    info!(
        "{} Create checkpoint, settle_version: {}, size: {:?}",
        env.log_prefix,
        game_context.settle_version(),
        checkpoint_size,
    );

    let accept_deposits = game_context.take_accept_deposits();

    ports
        .send(EventFrame::Checkpoint {
            access_version: game_context.access_version(),
            settle_version: game_context.settle_version(),
            previous_settle_version: original_versions.settle_version,
            checkpoint: checkpoint.clone(),
            settles,
            transfer,
            awards,
            state_sha: game_context.state_sha(),
            entry_lock,
            accept_deposits,
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
        info!("{} Launch subgame: {}", env.log_prefix, sub_game.id);
        let SubGame {
            bundle_addr,
            id,
            init_account,
        } = sub_game;
        // Use the existing checkpoint when possible.
        let spec = GameSpec {
            game_addr: game_context.game_addr().to_owned(),
            game_id: id,
            bundle_addr,
            max_players: init_account.max_players,
        };
        let versions = game_context.versions();
        let ef = EventFrame::LaunchSubGame {
            sub_game_init: Box::new(SubGameInit {
                spec,
                nodes: game_context.get_nodes().into(),
                source: SubGameInitSource::FromInitAccount(init_account, versions),
            }),
        };
        ports.send(ef).await;
    }
}

pub async fn init_state(
    access_version: u64,
    settle_version: u64,
    handler: &mut WrappedHandler,
    mut game_context: &mut GameContext,
    ports: &PipelinePorts,
    client_mode: ClientMode,
    game_mode: GameMode,
    env: &ComponentEnv,
) -> Option<CloseReason> {
    let init_account = match game_context.init_account() {
        Ok(init_account) => init_account,
        Err(e) => return Some(CloseReason::Fault(e)),
    };

    let original_versions = game_context.versions();

    let effects = match handler.init_state(&mut game_context, &init_account) {
        Ok(effects) => {
            print_logs(&effects.logs, env);
            effects
        }
        Err(e) => {
            error!("{} Failed to initialize state: {:?}", env.log_prefix, e);
            error!("{} Init Account: {:?}", env.log_prefix, init_account);
            ports.send(EventFrame::Shutdown).await;
            return Some(CloseReason::Fault(e));
        }
    };

    let EventEffects {
        checkpoint, ..
    } = effects;

    info!(
        "{} Initialize game state, access_version: {}, settle_version: {}, SHA: {}",
        env.log_prefix,
        access_version,
        settle_version,
        game_context.state_sha()
    );

    let Some(checkpoint) = checkpoint else {
        ports.send(EventFrame::Shutdown).await;
        return Some(CloseReason::Fault(Error::CheckpointNotFoundAfterInit));
    };

    send_settlement(
        None,
        vec![],
        vec![],
        None,
        original_versions,
        &mut game_context,
        ports,
        env,
    )
    .await;

    // Dispatch the initial Ready event if running in Transactor mode.
    if client_mode == ClientMode::Transactor {
        game_context.dispatch_safe(Event::Ready, 0);
    }

    // Tell master game the subgame is successfully created.
    if game_mode == GameMode::Sub {
        let game_id = game_context.game_id();
        let checkpoint_state = checkpoint.get_versioned_data(game_id);
        if let Some(checkpoint_state) = checkpoint_state {
            ports
                .send(EventFrame::SubGameReady {
                    checkpoint_state: checkpoint_state.clone(),
                    game_id: game_context.game_id(),
                })
                .await;
        } else {
            ports.send(EventFrame::Shutdown).await;
            return Some(CloseReason::Fault(Error::CheckpointNotFoundAfterInit));
        }
    }

    return None;
}

pub async fn resume_from_checkpoint(
    game_context: &mut GameContext,
    ports: &PipelinePorts,
    client_mode: ClientMode,
    game_mode: GameMode,
    env: &ComponentEnv,
) -> Option<CloseReason> {
    if game_mode == GameMode::Main {
        let versioned_data_list = game_context.checkpoint().list_versioned_data();
        info!(
            "{} Resume from checkpoint with {} subgames",
            env.log_prefix,
            versioned_data_list.len() - 1
        ); // except the master game

        for versioned_data in versioned_data_list {
            if versioned_data.id == 0 {
                continue;
            }
            let ef = EventFrame::LaunchSubGame {
                sub_game_init: Box::new(SubGameInit {
                    spec: GameSpec {
                        game_addr: game_context.game_addr().to_owned(),
                        game_id: versioned_data.id,
                        bundle_addr: versioned_data.game_spec.bundle_addr.clone(),
                        max_players: versioned_data.game_spec.max_players,
                    },
                    nodes: game_context.get_nodes().into(),
                    source: SubGameInitSource::FromCheckpoint(versioned_data.clone()),
                }),
            };
            ports.send(ef).await;
        }
    }

    if client_mode == ClientMode::Transactor {
        game_context.dispatch_safe(Event::Ready, 0);
    }

    None
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
            let EventEffects {
                settles,
                transfer,
                awards,
                checkpoint,
                launch_sub_games,
                bridge_events,
                start_game,
                entry_lock,
                logs,
                reject_deposits,
            } = effects;

            print_logs(&logs, env);

            // Broacast the event to clients
            if client_mode == ClientMode::Transactor {
                broadcast_event(event, &game_context, ports).await;
            }

            // Update the local client
            update_local_client(&game_context, ports).await;

            // Start game
            if client_mode == ClientMode::Transactor && start_game {
                send_start_game(&game_context, ports).await;
            }

            // Launch sub games
            if game_mode == GameMode::Main {
                launch_sub_game(launch_sub_games, game_context, ports, env).await;
            }

            if !reject_deposits.is_empty() {
                send_reject_deposits(reject_deposits, ports, env).await;
            }

            if checkpoint.is_some() {
                send_settlement(
                    transfer,
                    settles,
                    awards,
                    entry_lock,
                    original_versions,
                    game_context,
                    ports,
                    env,
                )
                .await;
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
                    ports.send(EventFrame::Shutdown).await;
                    return Some(CloseReason::Fault(e));
                }
                _ => (),
            }
        }
    }
    None
}

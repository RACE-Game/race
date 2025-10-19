use super::misc::log_execution_context;
use race_api::{
    effect::{EmitBridgeEvent, Log, SubGame},
    event::Event,
    prelude::InitAccount,
};
use race_core::{
    checkpoint::{Checkpoint, VersionedData},
    context::{EventEffects, GameContext, SubGameInit, SubGameInitSource},
    error::Error,
    types::{ClientMode, GameMode, GameSpec},
};
use race_transactor_frames::EventFrame;
use crate::{common::PipelinePorts, handler::HandlerT, CloseReason, ComponentEnv};
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

async fn broadcast_event(event: Event, game_context: &GameContext, ports: &PipelinePorts) {
    ports
        .send(EventFrame::Broadcast {
            event,
            timestamp: game_context.get_timestamp(),
            state_sha: game_context.state_sha(),
        })
        .await;
}

async fn send_checkpoint(
    checkpoint: Option<Checkpoint>,
    game_context: &GameContext,
    ports: &PipelinePorts,
) {
    if let Some(checkpoint) = checkpoint {
        ports
            .send(EventFrame::Checkpoint {
                checkpoint,
                access_version: game_context.access_version(),
                settle_version: game_context.settle_version(),
                state_sha: game_context.state_sha(),
                nodes: game_context.get_nodes().to_owned(),
            })
            .await;
    }
}

async fn send_context_updated(game_context: &GameContext, ports: &PipelinePorts) {
    ports
        .send(EventFrame::ContextUpdated {
            context: Box::new(game_context.clone()),
        })
        .await;
}

async fn send_subgame_ready(
    versioned_data: VersionedData,
    game_context: &GameContext,
    init_account: InitAccount,
    ports: &PipelinePorts,
) {
    ports
        .send(EventFrame::SubGameReady {
            versioned_data,
            game_id: game_context.game_id(),
            max_players: game_context.max_players(),
            init_data: init_account.data,
        })
        .await;
}

async fn send_subgame_recovered(game_id: usize, ports: &PipelinePorts) {
    ports.send(EventFrame::SubGameRecovered { game_id }).await;
}

pub async fn send_subgame_shutdown(
    game_id: usize,
    versioned_data: &VersionedData,
    ports: &PipelinePorts,
) {
    ports
        .send(EventFrame::SubGameShutdown {
            game_id,
            versioned_data: versioned_data.clone(),
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
            "{} Send bridge event, dest: {}, from: {}",
            env.log_prefix,
            be.dest,
            game_context.game_id()
        );
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
            versioned_data: checkpoint_state.clone(),
        };
        ports.send(ef).await;
    }
}

async fn send_reject_deposits(
    reject_deposits: Vec<u64>,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    info!(
        "{} Send reject deposits, {:?}",
        env.log_prefix, reject_deposits
    );

    let ef = EventFrame::RejectDeposits { reject_deposits };

    ports.send(ef).await;
}

async fn do_send_settlements(
    game_context: &mut GameContext,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    while let Some(settle_details) = game_context.take_first_ready_settle_details() {
        info!(
            "{} Send settlement, settle_version: {}",
            env.log_prefix,
            game_context.settle_version(),
        );

        // if game_context.game_id() == 0 {
        //     settle_details.print("do_send_settlements".to_string());
        // }

        ports
            .send(EventFrame::Settle {
                settle_details: Box::new(settle_details),
            })
            .await;
    }
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
    handler: &mut dyn HandlerT,
    mut game_context: &mut GameContext,
    ports: &PipelinePorts,
    _client_mode: ClientMode,
    game_mode: GameMode,
    env: &ComponentEnv,
) -> Option<CloseReason> {
    let init_account = game_context.init_account();

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

    info!(
        "{} Initialize game state, access_version: {}, settle_version: {}, SHA: {}",
        env.log_prefix,
        access_version,
        settle_version,
        game_context.state_sha()
    );

    let Some(checkpoint) = effects.checkpoint else {
        ports.send(EventFrame::Shutdown).await;
        return Some(CloseReason::Fault(Error::CheckpointNotFoundAfterInit));
    };

    send_checkpoint(Some(checkpoint.clone()), game_context, ports).await;

    do_send_settlements(game_context, ports, env).await;

    // Tell master game the subgame is successfully created.
    if game_mode == GameMode::Sub {
        let game_id = game_context.game_id();
        if let Some(versioned_data) = checkpoint.get_versioned_data(game_id) {
            println!("init sub game state");
            send_subgame_ready(versioned_data.clone(), game_context, init_account, ports).await;
        } else {
            ports.send(EventFrame::Shutdown).await;
            return Some(CloseReason::Fault(Error::CheckpointNotFoundAfterInit));
        }
    }

    return None;
}

pub async fn recover_from_checkpoint(
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
        ); // counting excludes the master game

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
        if game_mode == GameMode::Sub {
            send_subgame_recovered(game_context.game_id(), ports).await;
        }

        if game_mode == GameMode::Main {
            launch_sub_game(
                game_context.checkpoint().get_launch_subgames(),
                game_context,
                ports,
                env,
            )
            .await;
        }

        if let Some(versioned_data) = game_context
            .checkpoint()
            .list_versioned_data()
            .iter()
            .find(|vd| vd.id == game_context.game_id())
        {
            if !versioned_data.bridge_events.is_empty() {
                send_bridge_event(
                    versioned_data.bridge_events.clone(),
                    game_context,
                    ports,
                    env,
                )
                .await;
            }

            game_context.set_dispatch(versioned_data.dispatch.clone());
        }
    }

    None
}

pub async fn handle_event(
    handler: &mut dyn HandlerT,
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

    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            let EventEffects {
                launch_sub_games,
                bridge_events,
                stop_game,
                logs,
                reject_deposits,
                checkpoint,
                ..
            } = effects;

            print_logs(&logs, env);

            // Broacast the event to clients
            if client_mode == ClientMode::Transactor {
                broadcast_event(event, &game_context, ports).await;
            }

            send_checkpoint(checkpoint, game_context, ports).await;

            // Send ContextUpdated frame to update the local client
            // The local client sends necessary encryption/decryption
            // messages to transactor.
            send_context_updated(&game_context, ports).await;

            if client_mode == ClientMode::Transactor && stop_game {
                let game_id = game_context.game_id();
                if let Some(vd) = game_context.checkpoint().get_versioned_data(game_id) {
                    send_subgame_shutdown(game_id, vd, ports).await;
                }
            }

            // Launch sub games
            if game_mode == GameMode::Main {
                launch_sub_game(launch_sub_games, game_context, ports, env).await;
            }

            if !reject_deposits.is_empty() {
                send_reject_deposits(reject_deposits, ports, env).await;
            }

            do_send_settlements(game_context, ports, env).await;

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

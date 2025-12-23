use super::misc::log_execution_context;
use race_api::{
    effect::{Effect, EmitBridgeEvent, Log, LaunchSubGame},
    event::Event,
    prelude::InitAccount,
};
use race_core::{
    node::Node,
    versions::Versions,
    checkpoint::{VersionedData, SharedData, ContextCheckpoint},
    context::{GameContext, EventEffects},
    error::Error,
    game_spec::GameSpec,
    engine::general_handle_event,
    types::{ClientMode, GameMode},
};
use race_transactor_frames::EventFrame;
use crate::{common::PipelinePorts, CloseReason, ComponentEnv};
use tracing::{debug, error, info, warn};
use race_handler::{HandlerT, HandlerManager};
use race_core::encryptor::EncryptorT;
use race_core::entry_type::EntryType;

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

async fn broadcast_event(event: Event, context: &GameContext, ports: &PipelinePorts) {
    ports
        .send(EventFrame::Broadcast {
            event,
            timestamp: context.get_timestamp(),
            state_sha: context.state_sha(),
        })
        .await;
}

async fn send_checkpoint(
    checkpoint: ContextCheckpoint,
    ports: &PipelinePorts,
) {
    let f = EventFrame::Checkpoint {
        checkpoint,
    };
    ports.send(f).await;
}

async fn update_local_client(game_context: &GameContext, ports: &PipelinePorts) {
    ports
        .send(EventFrame::ContextUpdated {
            context: Box::new(game_context.clone()),
        })
        .await;
}

#[allow(unused)]
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

#[allow(unused)]
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
    versioned_data: &VersionedData,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    for be in bridge_events {
        let from_game_id = versioned_data.game_spec.game_id;
        info!(
            "{} Send bridge event, dest: {}, from: {}",
            env.log_prefix,
            be.dest,
            from_game_id,
        );
        let ef = EventFrame::SendBridgeEvent {
            from: from_game_id,
            dest: be.dest,
            event: Event::Bridge {
                dest_game_id: be.dest,
                from_game_id: from_game_id,
                raw: be.raw,
            },
            versioned_data: versioned_data.clone(),
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

/// Launch a subgame from the parameters.
/// Use this function to initialize a subgame.
// pub struct LaunchSubGame {
//     pub id: usize,
//     pub bundle_addr: String,
//     pub init_account: InitAccount,
// }
async fn launch_sub_game(
    game_addr: String,
    launch_sub_game: LaunchSubGame,
    shared_data: &SharedData,
    handler_manager: &mut HandlerManager,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) -> Result<(), Error> {

    let LaunchSubGame {
        id, bundle_addr, init_account
    } = launch_sub_game;

    // Create a temporary handler to initialize the first checkpoint state.
    let mut handler = handler_manager.get_handler(&bundle_addr).await?;

    let game_spec = GameSpec {
        game_addr,
        game_id: id,
        bundle_addr,
        max_players: init_account.max_players,
        entry_type: EntryType::Disabled, // Subgame's entry type is always disabled
    };

    let effect = init_state_with_init_account(
        &mut *handler,
        &init_account,
        ports,
        env,
    ).await?;

    let Some(handler_state) = effect.handler_state else {
        return Err(Error::InvalidHandlerState);
    };

    let versioned_data = VersionedData::new(game_spec, Versions::new(1, 1), handler_state);

    launch_sub_game_from_versioned_data(&versioned_data, &shared_data, ports, env).await;

    Ok(())
}

// XXX, should we load the bundle here?
/// Lanuch a subgame from versioned data.
/// Use this function to send a frame to recover a subgame.
async fn launch_sub_game_from_versioned_data(
    versioned_data: &VersionedData,
    shared_data: &SharedData,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) {
    info!("{} Launch subgame: {}", env.log_prefix, versioned_data.game_spec.game_id);

    let ef = EventFrame::LaunchSubGame {
        checkpoint: Box::new(
            ContextCheckpoint::new(shared_data.clone(), versioned_data.clone())
        ),
    };

    ports.send(ef).await;
}

async fn init_state_with_init_account(
    handler: &mut dyn HandlerT,
    init_account: &InitAccount,
    ports: &PipelinePorts,
    env: &ComponentEnv,
) -> Result<Effect, Error> {
    println!("Initialize handler state with InitAccount: {:?}", init_account);
    let mut effect = match handler.init_state(&init_account) {
        Ok(effect) => {
            print_logs(&effect.logs, env);
            effect
        }
        Err(e) => {
            error!("{} Failed to initialize state: {:?}", env.log_prefix, e);
            error!("{} Init Account: {:?}", env.log_prefix, init_account);
            ports.send(EventFrame::Shutdown).await;
            return Err(e);
        }
    };

    if let Some(e) = effect.__take_error() {
        return Err(e.into());
    }

    Ok(effect)
}

/// This function is only called once when the master game is initialized at its first time.
pub async fn init_state(
    access_version: u64,
    settle_version: u64,
    nodes: Vec<Node>,
    handler: &mut dyn HandlerT,
    game_spec: &GameSpec,
    init_account: InitAccount,
    ports: &PipelinePorts,
    _game_mode: GameMode,
    env: &ComponentEnv,
) -> Result<GameContext, Error> {

    let effect = init_state_with_init_account(
        handler,
        &init_account,
        ports,
        env,
    ).await?;

    let Some(ref handler_state) = effect.handler_state else {
        return Err(Error::InvalidHandlerState);
    };
    let balances = vec![];
    let shared_data = SharedData::new(balances, nodes);
    let versions = Versions::new(access_version, settle_version);
    let versioned_data = VersionedData::new(game_spec.clone(), versions, handler_state.clone());
    let mut game_context = match GameContext::try_new(shared_data, versioned_data) {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    game_context.apply_effect(effect)?;

    info!(
        "{} Initialize game state, versions: A#{} S#{}, SHA: {}",
        env.log_prefix,
        game_context.access_version(),
        game_context.settle_version(),
        game_context.state_sha()
    );

    let checkpoint = game_context.checkpoint();

    send_checkpoint(checkpoint, ports).await;

    do_send_settlements(&mut game_context, ports, env).await;

    // XXX we will never initialize the subgame through this function
    // the subgame is always initialized from checkpoint.
    //
    // // Tell master game the subgame is successfully created.
    // if game_mode == GameMode::Sub {
    //     let game_id = game_context.game_id();
    //     if let Some(versioned_data) = checkpoint.get_versioned_data(game_id) {
    //         println!("init sub game state");
    //         send_subgame_ready(versioned_data.clone(), game_context, init_account, ports).await;
    //     } else {
    //         ports.send(EventFrame::Shutdown).await;
    //         return Some(CloseReason::Fault(Error::CheckpointNotFoundAfterInit));
    //     }
    // }

    return Ok(game_context);
}

pub async fn recover_from_checkpoint(
    checkpoint: &ContextCheckpoint,
    ports: &PipelinePorts,
    client_mode: ClientMode,
    game_mode: GameMode,
    env: &ComponentEnv,
) -> Result<GameContext, Error> {
    // The root data is the VersionedData for this game.
    // And this VersionedData may contain more VersionedDatas of sub games.
    // We only resume sub games when we are at Transactor mode.
    if game_mode == GameMode::Main && client_mode == ClientMode::Transactor {
        info!(
            "{} Resume from checkpoint with {} subgames.",
            env.log_prefix,
            checkpoint.root_data.sub_data.len(),
        );

        for versioned_data in checkpoint.root_data.sub_data.values() {
            launch_sub_game_from_versioned_data(versioned_data, checkpoint.shared_data(), ports, env).await;
        }
    }

    // let Some(versioned_data) = checkpoint
    //     .list_versioned_data()
    //     .iter()
    //     .find(|vd| vd.game_spec.game_id == game_id) else {
    //         return Err(Error::InvalidGameId);
    //     };

    let versioned_data = checkpoint.root_data();

    // Redispatch all unhandled bridge events.
    if client_mode == ClientMode::Transactor && !versioned_data.bridge_events.is_empty() {
        send_bridge_event(
            versioned_data.bridge_events.clone(),
            &versioned_data,
            ports,
            env,
        ).await;
    }

    // XXX why do this?
    // game_context.set_dispatch(versioned_data.dispatch.clone());

    let game_context = match GameContext::try_new(
        checkpoint.shared_data().to_owned(),
        versioned_data.to_owned(),
    ) {
        Ok(game_context) => game_context,
        Err(e) => return Err(e),
    };

    Ok(game_context)
}

pub async fn handle_event(
    handler: &mut dyn HandlerT,
    handler_manager: &mut HandlerManager,
    game_context: &mut GameContext,
    event: Event,
    encryptor: &dyn EncryptorT,
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

    // Clone the game context to make sure the whole procedure is atomic

    let mut new_game_context = game_context.clone();

    new_game_context.set_timestamp(timestamp);

    // Apply general handler

    if let Err(e) = general_handle_event(
        &mut new_game_context, &event, encryptor,
    ) {
        return Some(CloseReason::Fault(e));
    };

    // Prepare the input effect, apply it game handler

    let effect = new_game_context.derive_effect();

    let effect = match  handler.handle_event(&effect, &event) {
        Ok(eff) => eff,
        Err(e) => {
            warn!("{} Handle event error: {}", env.log_prefix, e.to_string());
            log_execution_context(&new_game_context, &event);
            match e {
                Error::WasmExecutionError(_) | Error::WasmMemoryOverflow => {
                    ports.send(EventFrame::Shutdown).await;
                    return Some(CloseReason::Fault(e));
                }
                _ => {
                    return None;
                },
            }
        }
    };

    // Parse the output effect

    let event_effects = match new_game_context.apply_effect(effect) {
        Ok(ee) => ee,
        Err(_e) => {
            // The handler encounters an normal error?
            return None;
        }
    };

    let EventEffects {
        launch_sub_games,
        bridge_events,
        stop_game,
        logs,
        reject_deposits,
        ..
    } = event_effects;

    print_logs(&logs, env);

    // Broacast the event to clients
    if client_mode == ClientMode::Transactor {
        broadcast_event(event, &new_game_context, ports).await;
    }

    // XXX, where to build this checkpoint.
    send_checkpoint(new_game_context.checkpoint().clone(), ports).await;

    // Update the local client
    update_local_client(&new_game_context, ports).await;

    if client_mode == ClientMode::Transactor && stop_game {
        let game_id = new_game_context.game_id();
        send_subgame_shutdown(game_id, new_game_context.versioned_data(), ports).await;
    }

    // Launch sub games
    if game_mode == GameMode::Main {
        for lsg in launch_sub_games {
            if let Err(e) = launch_sub_game(
                new_game_context.game_addr().to_string(),
                lsg,
                new_game_context.checkpoint().shared_data(),
                handler_manager,
                ports,
                env
            ).await {
                return Some(CloseReason::Fault(e));
            }
        }
    }

    if !reject_deposits.is_empty() {
        send_reject_deposits(reject_deposits, ports, env).await;
    }

    do_send_settlements(&mut new_game_context, ports, env).await;

    // Emit bridge events
    // XXX how to get this versioned data
    if client_mode == ClientMode::Transactor {
        send_bridge_event(bridge_events, new_game_context.versioned_data(), ports, env).await;
    }

    *game_context = new_game_context;

    None
}

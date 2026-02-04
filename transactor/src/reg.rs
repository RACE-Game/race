//! Register current transactor into on-chain transactor list
//! Find available games and serve them.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use race_core::error::{Error, Result};
use race_core::types::ClientMode;
use race_core::{
    transport::TransportT,
    types::{RegisterServerParams, ServeParams},
};
use race_encryptor::generate_credentials;
use race_env::Config;
use race_transport::TransportBuilder;
use tokio::select;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use race_transactor_frames::SignalFrame;
use crate::context::ApplicationContext;

/// Register current server.
pub async fn register_server(config: &Config) -> Result<()> {
    let transactor_conf = config
        .transactor
        .as_ref()
        .ok_or(Error::TransactorConfigMissing)?;
    let transport: Box<dyn TransportT> = TransportBuilder::default()
        .with_chain_by_name(transactor_conf.chain.as_str())
        .try_with_config(config)?
        .build()
        .await?;

    let secret = transport.generate_secret().await?;
    let credentials = generate_credentials(secret)?;
    let credentials = borsh::to_vec(&credentials)?;

    info!("Transport built successfully");
    transport
        .register_server(RegisterServerParams {
            endpoint: transactor_conf.endpoint.to_owned(),
            credentials,
        })
        .await?;
    info!("Server account created");
    Ok(())
}

/// Start the registration task.
/// This task will scan the games in registration account, find unserved games and join.
pub async fn start_reg_task(context: &ApplicationContext) -> JoinHandle<()> {
    let blacklist = context.blacklist();
    let mut shutdown_rx = context.get_shutdown_receiver();

    let (reg_addresses, transport, server_addr, signal_tx) = {
        (
            context.config.reg_addresses.clone(),
            context.transport.clone(),
            context.account.addr.clone(),
            context.get_signal_sender(),
        )
    };
    info!("Server address: {}", server_addr);
    info!("Registraion addresses: {:?}", reg_addresses);

    tokio::spawn(async move {
        let mut not_found_counts = HashMap::<String, usize>::new();

        let server_addr = if let Ok(Some(server_account)) = transport.get_server_account(&server_addr).await {
            server_account.addr
        } else {
            error!("Server account for [{}] not found, please run `reg` command first.", server_addr);
            return;
        };

        let mut loaded_game_addrs: HashSet<String> = Default::default();

        loop {
            // We search for accounts every 10 seconds
            for addr in reg_addresses.iter() {
                if let Ok(Some(reg)) = transport.get_registration(addr).await {
                    for game_reg in reg.games.into_iter() {
                        if blacklist.lock().await.contains_addr(&game_reg.addr) {
                            continue;
                        }
                        if loaded_game_addrs.contains(&game_reg.addr) {
                            continue;
                        }
                        match transport.get_game_account(&game_reg.addr).await {
                            Ok(Some(game_account)) => {

                                // Check if we are registered
                                if !game_account.servers.iter().any(|s| s.addr.eq(&server_addr)) {
                                    // Register to game
                                    let register_result = transport.serve(ServeParams {
                                        game_addr: game_account.addr.clone(),
                                    }).await;

                                    if let Err(e) = register_result {
                                        error!("Failed to register to game account at [{}] due to {:?}", game_reg.addr, e);
                                    }

                                }

                                let Ok(Some(game_account)) = transport.get_game_account(&game_account.addr).await else {
                                    error!("Failed to fetch game account after sending a serve transaction");
                                    continue;
                                };

                                let Some(transactor_addr) = game_account.transactor_addr.as_ref() else {
                                    error!("Failed to find transactor addr after sending a serve transaction");
                                    continue;
                                };

                                loaded_game_addrs.insert(game_account.addr.clone());

                                let mode = if transactor_addr.eq(&server_addr) {
                                    ClientMode::Transactor
                                } else {
                                    ClientMode::Validator
                                };
                                let signal_result = signal_tx
                                    .send(SignalFrame::StartGame {
                                        game_addr: game_account.addr.clone(),
                                        mode,
                                    })
                                    .await;

                                if let Err(e) = signal_result {
                                    error!("Failed to send StartGame for [{}] signal due to {:?}", game_reg.addr, e);
                                }
                            }
                            Ok(None) => {
                                warn!("Game account not found: {:?}", &game_reg.addr);
                                match not_found_counts.entry(game_reg.addr.to_string()) {
                                    std::collections::hash_map::Entry::Occupied(mut cnt) => {
                                        *cnt.get_mut() += 1;
                                        if *cnt.get() == 2 {
                                            blacklist.lock().await.add_addr(&game_reg.addr);
                                        }
                                    }
                                    std::collections::hash_map::Entry::Vacant(cnt) => {
                                        cnt.insert(0);
                                    }
                                }

                            }
                            Err(e) => {
                                error!("Failed to fetch game account due to {:?}", e);
                            }
                        }
                    }
                } else {
                    warn!("Failed to load registration at {}", addr);
                }
            }

            select! {
                _ = shutdown_rx.changed() => {
                    info!("Stop discovering games");
                    break;
                },
                _ = tokio::time::sleep(Duration::from_secs(10)) => { continue; }
            };
        }
    })
}

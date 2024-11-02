//! Register current transactor into on-chain transactor list
//! Find available games and serve them.

use std::{collections::HashMap, time::Duration};

use race_api::error::{Error, Result};
use race_core::{
    transport::TransportT,
    types::{RegisterServerParams, ServeParams},
};
use race_env::Config;
use race_transport::TransportBuilder;
use tracing::{error, info, warn};

use crate::{context::ApplicationContext, frame::SignalFrame};

/// Register current server.
pub async fn register_server(config: &Config) -> Result<()> {
    let transactor_conf = config
        .transactor
        .as_ref()
        .ok_or(Error::TransactorConfigMissing)?;
    let transport: Box<dyn TransportT> = TransportBuilder::default()
        .try_with_chain(transactor_conf.chain.as_str())?
        .try_with_config(config)?
        .build()
        .await?;
    info!("Transport built successfully");
    transport
        .register_server(RegisterServerParams {
            endpoint: transactor_conf.endpoint.to_owned(),
        })
        .await?;
    info!("Server account created");
    Ok(())
}

/// Start the registration task.
/// This task will scan the games in registration account, find unserved games and join.
pub async fn start_reg_task(context: &ApplicationContext) {
    let key = context.export_public_key();
    let blacklist = context.blacklist();

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

        loop {
            // We search for accounts every 10 seconds
            for addr in reg_addresses.iter() {
                if let Ok(Some(reg)) = transport.get_registration(addr).await {
                    for game_reg in reg.games.into_iter() {
                        let mode = race_core::types::QueryMode::Finalized;
                        if blacklist.lock().await.contains_addr(&game_reg.addr) {
                            continue;
                        }
                        match transport.get_game_account(&game_reg.addr, mode).await {
                            Ok(Some(game_account)) => {
                                // Check if we are registered
                                if !game_account.servers.iter().any(|s| s.addr.eq(&server_addr)) {
                                    // Register to game
                                    let register_result = transport.serve(ServeParams {
                                        game_addr: game_account.addr.clone(),
                                        verify_key: key.ec.clone(),
                                    }).await;

                                    if let Err(e) = register_result {
                                        error!("Failed to register to game account at [{}] due to {:?}", game_reg.addr, e);
                                    }

                                }

                                let signal_result = signal_tx
                                    .send(SignalFrame::StartGame {
                                        game_addr: game_account.addr.clone(),
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
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
}

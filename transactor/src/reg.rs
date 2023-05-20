//! Register current transactor into on-chain transactor list
//! Find available games and serve them.

use std::time::Duration;

use race_core::{
    error::{Error, Result},
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
        loop {
            // We search for accounts every 10 seconds
            for addr in reg_addresses.iter() {
                if let Ok(Some(reg)) = transport.get_registration(addr).await {
                    for game_reg in reg.games.into_iter() {
                        if let Some(game_account) =
                            transport.get_game_account(&game_reg.addr).await.unwrap()
                        {
                            // We will keep registering until we become the transactor.
                            if game_account
                                .servers
                                .iter()
                                .find(|s| s.addr.eq(&server_addr))
                                .is_none()
                            {
                                let server_account =
                                    transport.get_server_account(&server_addr).await.unwrap();
                                if server_account.is_none() {
                                    error!(
                                        "Server account not found, please run `task` command first"
                                    );
                                    return;
                                }
                                info!("Serve game at {:?}", game_account.addr);
                                if let Err(e) = transport
                                    .serve(ServeParams {
                                        game_addr: game_account.addr.clone(),
                                        verify_key: key.ec.clone(),
                                    })
                                    .await
                                {
                                    error!("Error serve game: {:?}", e)
                                }
                            }

                            let r = signal_tx
                                .send(SignalFrame::StartGame {
                                    game_addr: game_account.addr.clone(),
                                })
                                .await;
                            if let Err(e) = r {
                                error!(
                                    "Failed to send signal to start game {}: {:?}",
                                    game_account.addr, e
                                );
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

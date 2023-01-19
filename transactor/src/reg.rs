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
use tokio::sync::Mutex;
use tracing::info;

use crate::context::ApplicationContext;

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
    let addr = transport
        .register_server(RegisterServerParams {
            endpoint: transactor_conf.endpoint.to_owned(),
        })
        .await?;
    info!("Server account created at {:?}", addr);
    Ok(())
}

/// Start the registration task.
/// This task will scan the games in registration account, find unserved games and join.
pub async fn start_reg_task(context: &Mutex<ApplicationContext>) {
    let (reg_addresses, transport, server_addr) = {
        let context = context.lock().await;
        (
            context.config.reg_addresses.clone(),
            context.transport.clone(),
            context.account.addr.clone(),
        )
    };
    info!("Registraion addresses: {:?}", reg_addresses);
    tokio::spawn(async move {
        loop {
            // We search for accounts every 10 seconds
            tokio::time::sleep(Duration::from_secs(10)).await;
            for addr in reg_addresses.iter() {
                if let Some(reg) = transport.get_registration(addr).await {
                    for game_reg in reg.games.into_iter() {
                        if let Some(game_account) = transport.get_game_account(&game_reg.addr).await
                        {
                            if game_account.transactor_addr.is_none()
                                || matches!(game_account.transactor_addr, Some(x) if x.ne(&server_addr))
                            {
                                info!("Serve game at {:?}", game_account.addr);
                                if let Ok(_) = transport
                                    .serve(ServeParams {
                                        game_addr: game_account.addr.clone(),
                                        server_addr: server_addr.clone(),
                                    })
                                    .await
                                {
                                    info!("Game account at {:?}, updated", game_account.addr);
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

// /// Start registration loop.
// pub async fn start_self_registration(
//     config: &Config,
//     transport: Arc<dyn TransportT>,
// ) -> Result<()> {
//     let transactor_conf = &config.transactor.expect("Missing transactor configuration");

//     // let transport = create_transport(config, chain)?;
//     // match config.transactor {
//     //     Some(ref conf) => {
//     //         let transport = create_transport(config, &conf.chain).expect("Failed to create transport");
//     //         let params = RegisterTransactorParams {
//     //             endpoint: conf.endpoint.clone(),
//     //             owner_addr: "".into(),
//     //         };
//     //         loop {
//     //             transport.register_transactor(params.clone()).await.expect("Failed to register");
//     //         }
//     //     }
//     //     _ => panic!("Missing transactor configuration"),
//     // }
// }

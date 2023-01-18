//! Register current transactor into on-chain transactor list
//! Find available games and serve them.

use race_core::{
    error::{Error, Result},
    transport::TransportT,
    types::RegisterServerParams,
};
use race_env::Config;
use race_transport::TransportBuilder;

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
    transport
        .register_server(RegisterServerParams {
            endpoint: transactor_conf.endpoint.to_owned(),
        })
        .await?;
    Ok(())
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

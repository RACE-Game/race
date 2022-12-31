//! Register current transactor into on-chain transactor list
//! Find available games and serve them.

use race_core::{error::Result, types::RegisterTransactorParams};
use race_env::Config;
use race_transport::create_transport;

/// Start registration loop.
pub async fn start_self_registration(config: &Config) -> Result<()> {
    // let transactor_conf = &config.transactor.expect("Missing transactor configuration");
    // let transport = create_transport(config, chain)?;
    match config.transactor {
        Some(ref conf) => {
            let transport = create_transport(config, &conf.chain).expect("Failed to create transport");
            let params = RegisterTransactorParams {
                endpoint: conf.endpoint.clone(),
                owner_addr: "".into(),
            };
            loop {
                transport.register_transactor(params.clone()).await.expect("Failed to register");
            }
        }
        _ => panic!("Missing transactor configuration"),
    }
}

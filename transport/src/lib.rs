use std::sync::Arc;

use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_env::Config;

pub mod evm;
pub mod facade;
pub mod mock;
pub mod solana;

pub fn create_transport(config: &Config, chain: &str) -> Result<Arc<dyn TransportT>> {
    match chain {
        "facade" => {
            if let Some(ref params) = config.facade {
                let transport = facade::FacadeTransport::new(&params.host);
                Ok(Arc::new(transport))
            } else {
                Err(Error::ConfigMissing)
            }
        }
        "solana" => {
            if let Some(ref params) = config.solana {
                let transport = solana::SolanaTransport::new(&params.rpc);
                Ok(Arc::new(transport))
            } else {
                Err(Error::ConfigMissing)
            }
        }
        "bnb" => {
            if let Some(ref params) = config.bnb {
                let transport = evm::EvmTransport::new(&params.rpc);
                Ok(Arc::new(transport))
            } else {
                Err(Error::ConfigMissing)
            }
        }
        _ => Err(Error::InvalidChainName),
    }
}

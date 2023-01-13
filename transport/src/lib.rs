use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_env::Config;
// use signer::Signer;

pub mod evm;
pub mod facade;
pub mod signer;
pub mod solana;

pub async fn create_transport_for_app(chain: &str, rpc: &str) -> Result<Box<dyn TransportT>> {
    match chain {
        "facade" => Ok(Box::new(facade::FacadeTransport::new(rpc).await)),
        "solana" => Ok(Box::new(solana::SolanaTransport::new(rpc))),
        "bnb" => Ok(Box::new(evm::EvmTransport::new(rpc))),
        _ => Err(Error::InvalidChainName),
    }
}

pub async fn create_transport(config: &Config, chain: &str) -> Result<Box<dyn TransportT>> {
    match chain {
        "facade" => {
            if let Some(ref params) = config.facade {
                let transport = facade::FacadeTransport::new(&params.host).await;
                Ok(Box::new(transport))
            } else {
                Err(Error::ConfigMissing)
            }
        }
        "solana" => {
            if let Some(ref params) = config.solana {
                let transport = solana::SolanaTransport::new(&params.rpc);
                Ok(Box::new(transport))
            } else {
                Err(Error::ConfigMissing)
            }
        }
        "bnb" => {
            if let Some(ref params) = config.bnb {
                let transport = evm::EvmTransport::new(&params.rpc);
                Ok(Box::new(transport))
            } else {
                Err(Error::ConfigMissing)
            }
        }
        _ => Err(Error::InvalidChainName),
    }
}

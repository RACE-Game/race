pub mod error;
pub mod evm;
pub mod facade;
pub mod signer;
pub mod solana;

use error::{TransportError, TransportResult};
use race_core::transport::TransportT;
use race_env::Config;
use signer::Signer;
use tracing::info;

#[derive(Debug, PartialEq, Eq)]
pub enum ChainType {
    Solana,
    Bnb,
    Facade,
}

impl TryFrom<&str> for ChainType {
    type Error = TransportError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "bnb" => Ok(Self::Bnb),
            "facade" => Ok(Self::Facade),
            "solana" => Ok(Self::Solana),
            _ => Err(TransportError::InvalidChainName(value.into())),
        }
    }
}

#[derive(Default)]
pub struct TransportBuilder {
    chain: Option<ChainType>,
    rpc: Option<String>,
    signer: Option<Box<dyn Signer>>,
}

impl TransportBuilder {
    pub fn with_chain(mut self, chain: ChainType) -> Self {
        self.chain = Some(chain);
        self
    }

    pub fn try_with_chain<T>(mut self, chain: T) -> TransportResult<Self>
    where
        T: TryInto<ChainType, Error = TransportError>,
    {
        self.chain = Some(chain.try_into()?);
        Ok(self)
    }

    pub fn with_rpc<S: Into<String>>(mut self, rpc: S) -> Self {
        self.rpc = Some(rpc.into());
        self
    }

    pub fn with_signer(mut self, signer: Box<dyn Signer>) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn try_with_config(mut self, config: &Config) -> TransportResult<Self> {
        if let Some(ref chain) = self.chain {
            match chain {
                ChainType::Solana => {
                    self.rpc = Some(
                        config
                            .solana
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig)?
                            .rpc
                            .clone(),
                    );
                }
                ChainType::Bnb => {
                    self.rpc = Some(
                        config
                            .bnb
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig)?
                            .rpc
                            .clone(),
                    );
                }
                ChainType::Facade => {
                    self.rpc = Some(
                        config
                            .facade
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig)?
                            .host
                            .clone(),
                    );
                }
            }
            Ok(self)
        } else {
            Err(TransportError::UnspecifiedChain)
        }
    }

    pub async fn build(self) -> TransportResult<Box<dyn TransportT>> {
        if let Some(chain) = self.chain {
            match chain {
                ChainType::Solana => {
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    // let signer = self.signer.ok_or(TransportError::UnspecifiedSigner)?;
                    Ok(Box::new(solana::SolanaTransport::new(rpc)))
                }
                ChainType::Bnb => {
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    // let signer = self.signer.ok_or(TransportError::UnspecifiedSigner)?;
                    Ok(Box::new(evm::EvmTransport::new(rpc)))
                }
                ChainType::Facade => {
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    info!("Build FacadeTransport for {:?}", rpc);
                    Ok(Box::new(facade::FacadeTransport::try_new(&rpc).await?))
                }
            }
        } else {
            Err(TransportError::UnspecifiedChain)
        }
    }
}

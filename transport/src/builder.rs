use crate::chain_type::ChainType;
use crate::error::{TransportError, TransportResult};
use crate::facade;
use crate::solana;
use race_core::transport::TransportT;
use race_env::Config;
use std::path::PathBuf;
use tracing::info;

#[derive(Default)]
pub struct TransportBuilder {
    chain: Option<ChainType>,
    rpc: Option<String>,
    keyfile: Option<PathBuf>,
    address: Option<String>,
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

    pub fn with_keyfile<S: Into<PathBuf>>(mut self, keyfile: S) -> Self {
        self.keyfile = Some(keyfile.into());
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
                            .ok_or(TransportError::InvalidConfig("RPC unspecified".into()))?
                            .rpc
                            .clone(),
                    );
                    self.keyfile = Some(
                        config
                            .solana
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig("Keyfile not found".into()))?
                            .keyfile
                            .clone(),
                    );
                }
                ChainType::Bnb => {
                    self.rpc = Some(
                        config
                            .bnb
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig("RPC unspecified".into()))?
                            .rpc
                            .clone(),
                    );
                }
                ChainType::Facade => {
                    self.address = Some(config.facade.as_ref().ok_or(TransportError::InvalidConfig("Address unspecified".into()))?.address.clone());
                    self.rpc = Some(
                        config
                            .facade
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig("RPC unspecified".into()))?
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
                    let keyfile = self.keyfile.ok_or(TransportError::UnspecifiedSigner)?;
                    Ok(Box::new(solana::SolanaTransport::try_new(rpc, keyfile)?))
                }
                ChainType::Facade => {
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    let address = self.address.ok_or(TransportError::UnspecifiedSigner)?;
                    info!("Build FacadeTransport for {:?}", rpc);
                    Ok(Box::new(facade::FacadeTransport::try_new(address.to_owned(), &rpc).await?))
                }
                _ => unimplemented!(),
            }
        } else {
            Err(TransportError::UnspecifiedChain)
        }
    }
}

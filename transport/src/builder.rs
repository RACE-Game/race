use crate::error::{TransportError, TransportResult};
use crate::facade;
use crate::solana;
use race_core::chain::ChainType;
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

    // Solana
    skip_preflight: Option<bool>,
}

impl TransportBuilder {
    pub fn with_chain(mut self, chain: ChainType) -> Self {
        self.chain = Some(chain);
        self
    }

    pub fn with_chain_by_name<T>(mut self, chain: T) -> Self
    where
        T: Into<ChainType>,
    {
        self.chain = Some(chain.into());
        self
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
                    self.skip_preflight = config.solana.as_ref().and_then(|c| c.skip_preflight);
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
                ChainType::Sui => {
                    self.rpc = Some(
                        config
                            .sui
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig("RPC unspecified".into()))?
                            .rpc
                            .clone(),
                    );
                    self.keyfile = Some(
                        config
                            .sui
                            .as_ref()
                            .ok_or(TransportError::InvalidConfig("Keyfile not found".into()))?
                            .keyfile
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
                    let skip_preflight = self.skip_preflight.unwrap_or(false);
                    Ok(Box::new(solana::SolanaTransport::try_new(rpc, self.keyfile, skip_preflight)?))
                }
                ChainType::Sui => {
                    use crate::sui::{self, PACKAGE_ID};
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    Ok(Box::new(sui::SuiTransport::try_new(rpc, PACKAGE_ID, self.keyfile).await?))
                }
                ChainType::Facade => {
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    info!("Build FacadeTransport for {:?}", rpc);
                    Ok(Box::new(facade::FacadeTransport::try_new(self.address.to_owned(), &rpc).await?))
                }
                _ => unimplemented!(),
            }
        } else {
            Err(TransportError::UnspecifiedChain)
        }
    }
}

use crate::error::{TransportError, TransportResult};
use crate::facade_wasm;
use crate::solana_wasm;
use crate::wasm_trait::TransportLocalT;

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

    pub async fn build(self) -> TransportResult<Box<dyn TransportLocalT>> {
        if let Some(chain) = self.chain {
            match chain {
                ChainType::Solana => {
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    Ok(Box::new(solana_wasm::SolanaWasmTransport::try_new(rpc)?))
                }
                ChainType::Facade => {
                    let rpc = self.rpc.ok_or(TransportError::UnspecifiedRpc)?;
                    Ok(Box::new(facade_wasm::FacadeTransport::try_new(&rpc).await?))
                }
                _ => unimplemented!(),
            }
        } else {
            Err(TransportError::UnspecifiedChain)
        }
    }
}

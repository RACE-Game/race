use crate::error::TransportError;

#[derive(Debug, PartialEq, Eq)]
pub enum ChainType {
    Bnb,
    Facade,
    Solana,
    Sui,
}

impl TryFrom<&str> for ChainType {
    type Error = TransportError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "bnb" => Ok(Self::Bnb),
            "facade" => Ok(Self::Facade),
            "solana" => Ok(Self::Solana),
            "sui" => Ok(Self::Sui),
            _ => Err(TransportError::InvalidChainName(value.into())),
        }
    }
}

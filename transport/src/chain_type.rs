use crate::error::TransportError;

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

#[derive(Debug, PartialEq, Eq)]
pub enum ChainType {
    Bnb,
    Facade,
    Solana,
    Sui,
}

impl From<&str> for ChainType {
    fn from(value: &str) -> Self {
        match value {
            "bnb" => Self::Bnb,
            "facade" => Self::Facade,
            "solana" => Self::Solana,
            "sui" => Self::Sui,
            _ => panic!("Invalid chain specified: {}", value),
        }
    }
}

impl ToString for ChainType {
    fn to_string(&self) -> String {
        match self {
            Self::Bnb => "bnb".into(),
            Self::Sui => "sui".into(),
            Self::Solana => "solana".into(),
            Self::Facade => "facade".into(),
        }
    }
}

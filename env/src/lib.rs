mod config;

pub use config::{Config, TransactorConfig};

pub fn default_rpc(chain: &str) -> &str {
    match chain {
        "facade" => "http://127.0.0.1:12002",
        "solana" => "https://mainnet-beta.solana.com",
        _ => panic!("Chain not supported, missing RPC endpoint"),
    }
}

pub fn default_keyfile(chain: &str) -> Option<&str> {
    match chain {
        "facade" => None,
        "solana" => Some("~/.config/solana/id.json"),
        _ => panic!("Chain not supported, missing keyfile"),
    }
}

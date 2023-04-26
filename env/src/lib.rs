mod config;

pub use config::{Config, TransactorConfig};

pub fn default_rpc<'a>(chain: &'a str, env: Option<&'a str>) -> &'a str {
    match (chain, env) {
        ("facade", _) => "http://127.0.0.1:12002",
        ("solana", Some("mainnet")) => "https://api.mainnet-beta.solana.com",
        ("solana", Some("testnet")) => "https://api.testnet.solana.com",
        ("solana", Some("devnet")) => "https://api.devnet.solana.com",
        ("solana", Some("local")) => "http://127.0.0.1:8899",
        _ => panic!("Chain not supported, missing RPC endpoint"),
    }
}

pub fn default_keyfile(chain: &str) -> Option<String> {
    match chain {
        "facade" => None,
        "solana" => Some(shellexpand::tilde("~/.config/solana/id.json").to_string()),
        _ => panic!("Chain not supported, missing keyfile"),
    }
}

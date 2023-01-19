//! Configuration of application

use std::{path::PathBuf, fs::File, io::Read};

use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
pub struct FacadeConfig {
    pub host: String,
}

#[derive(Deserialize)]
pub struct SolanaConfig {
    pub rpc: String,
    pub keyfile: PathBuf,
    pub reg_center: String,
}

#[derive(Deserialize)]
pub struct BnbConfig {
    pub rpc: String,
    pub keyfile: PathBuf,
    pub reg_center: String,
}

#[derive(Deserialize)]
pub struct TransactorConfig {
    pub port: u32,
    pub endpoint: String,
    pub chain: String,
    pub address: String,
    pub reg_addresses: Vec<String>,
}

#[derive(Deserialize)]
pub struct Config {
    pub transactor: Option<TransactorConfig>,
    pub facade: Option<FacadeConfig>,
    pub solana: Option<SolanaConfig>,
    pub bnb: Option<BnbConfig>,
}

impl Config {
    pub async fn from_path(path: &PathBuf) -> Config {
        info!("Load configuration from {:?}", path);
        let mut buf = Vec::with_capacity(1024);
        let mut f = File::open(path).expect("Config file not found");
        f.read_to_end(&mut buf).expect("Failed to read config file");
        match toml::from_slice(&buf) {
            Ok(config) => config,
            Err(e) => {
                panic!("Invalid config file: {:?}", e.to_string())
            }
        }
    }
}

//! Configuration of application

use std::path::PathBuf;

use serde::Deserialize;
use tokio::{fs::File, io::AsyncReadExt};

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
    pub endpoint: String,
    pub chain: String,
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
        println!("Load configuration: {:?}", path);
        let mut buf = Vec::with_capacity(1024);
        let mut f = File::open(path).await.expect("Config file not found");
        f.read_to_end(&mut buf).await.expect("Failed to read config file");
        match toml::from_slice(&buf) {
            Ok(config) => config,
            Err(e) => {
                panic!("Invalid config file: {:?}", e.to_string())
            }
        }
    }
}

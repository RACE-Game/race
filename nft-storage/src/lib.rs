//! Utilities for writing/reading NFT storages

use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Network error: {0}")]
    IoError(String),

    #[error("Malformed metadata: {0}")]
    MalformedMetadata(String),
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::IoError(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Deserialize, Debug)]
pub struct PropertyFiles {
    pub uri: String,
    pub r#type: String,
}

#[derive(Deserialize, Debug)]
pub struct GameBundleProperties {
    pub files: Vec<PropertyFiles>,
    pub category: String,
}

#[derive(Deserialize, Debug)]
pub struct GameBundleNftMetadata {
    pub name: String,
    pub symbol: String,
    pub properties: GameBundleProperties,
}

pub async fn fetch_wasm_from_game_bundle(uri: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let m: GameBundleNftMetadata = client.get(uri).send().await?.json().await?;

    // Find the wasm link
    if let Some(wasm_file) = m.properties.files.iter().find(|p| p.r#type == "application/wasm") {
        println!("wasm_file");
        let wasm_bytes = client.get(&wasm_file.uri).send().await?.bytes().await?.to_vec();
        println!("wasm_bytes");
        Ok(wasm_bytes)
    } else {
        return Err(Error::MalformedMetadata("Can't find URI of WASM bundle".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_wasm_from_game_bundle() -> anyhow::Result<()> {
        let data = fetch_wasm_from_game_bundle("https://arweave.net/qXM8gjD1sxey90T-U8KPz_Cuj-APuq3hu34AIWz9SXc").await?;
        assert_ne!(data.len(), 0);
        Ok(())
    }
}

use serde::Deserialize;

use crate::error::{TransportError, TransportResult};

#[derive(Deserialize, Debug)]
pub struct PropertyFiles {
    pub uri: String,
    pub r#type: String,
}

#[derive(Deserialize, Debug)]
pub struct GameBundleProperties {
    pub files: Vec<PropertyFiles>,
    #[allow(unused)]
    pub category: String,
}

#[derive(Deserialize, Debug)]
pub struct GameBundleNftMetadata {
    #[allow(unused)]
    pub name: String,
    #[allow(unused)]
    pub symbol: String,
    pub properties: GameBundleProperties,
}

pub async fn fetch_wasm_from_game_bundle(uri: &str) -> TransportResult<Vec<u8>> {
    let client = reqwest::Client::new();
    let m: GameBundleNftMetadata = client.get(uri).send().await?.json().await?;

    // Find the wasm link
    if let Some(wasm_file) = m
        .properties
        .files
        .iter()
        .find(|p| p.r#type == "application/wasm")
    {
        let wasm_bytes = client
            .get(&wasm_file.uri)
            .send()
            .await?
            .bytes()
            .await?
            .to_vec();
        Ok(wasm_bytes)
    } else {
        Err(TransportError::MetadataDeserializeError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_fetch_wasm_from_game_bundle() -> anyhow::Result<()> {
        let data = fetch_wasm_from_game_bundle(
            "https://arweave.net/qXM8gjD1sxey90T-U8KPz_Cuj-APuq3hu34AIWz9SXc",
        )
        .await?;
        assert_ne!(data.len(), 0);
        Ok(())
    }
}

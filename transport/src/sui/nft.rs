use bcs;
use race_core::types::GameBundle;
use crate::error::{TransportError, TransportResult};
use serde::{Serialize, Deserialize};
use sui_sdk::types::{
    base_types::{ObjectID, SuiAddress},
};

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BundleObject {
    pub id: ObjectID,
    pub name: String,
    pub symbol: String,
    pub uri: String,                // bundle wasm url
    pub cover: String               // bundle cover image url
}

impl BundleObject {
    pub fn into_bundle(self, data: Vec<u8>) -> GameBundle {
        GameBundle {
            addr: self.id.to_hex_uncompressed(),
            uri: self.uri,
            name: self.name,
            data
        }
    }
}

pub async fn fetch_wasm_from_game_bundle(uri: &str) -> TransportResult<Vec<u8>> {
    let client = reqwest::Client::new();

    // Fetch the wasm
    let wasm_bytes = client.get(uri).send().await?.bytes().await?.to_vec();

    Ok(wasm_bytes)
}

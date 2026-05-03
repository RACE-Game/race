/// This HandlerManager caches the downloaded bundles, to avoid
/// unnecessary networking.

use crate::handler::HandlerT;
use crate::wasm_handler::WasmHandler;
use tracing::{info, error};
use std::collections::HashMap;
use race_core::error::{Error, Result};
use std::collections::hash_map::Entry;
use race_core::types::GameBundle;

use tokio::sync::Mutex;

pub struct HandlerManager {
    bundles: Mutex<HashMap<String, GameBundle>>,
}

impl HandlerManager {
    pub fn new() -> Self {
        let bundles = Mutex::new(HashMap::default());

        Self {
            bundles,
        }
    }

    pub async fn fetch_bundle(&self, key: &str) -> Result<GameBundle> {
        info!("HandlerManager: Loading bundle from {}", key);
        let client = reqwest::Client::new();
        let wasm_bytes = client
            .get(key)
            .send()
            .await
            .map_err(|e| {
                error!("HandlerManager: failed to request: {}", e.to_string());
                Error::GameBundleNotFound
            })?
            .error_for_status()
            .map_err(|e| {
                error!("HandlerManager: failed to request: {}", e.to_string());
                Error::GameBundleNotFound
            })?
            .bytes()
            .await
            .map_err(|e| {
                error!("HandlerManager: failed to parse response: {}", e.to_string());
                Error::MalformedGameBundle
            })?
            .to_vec();

        // TODO: Error handling
        return Ok(GameBundle { key: key.to_string(), data: wasm_bytes })
    }

    pub async fn get_handler(&self, bundle_key: &str) -> Result<Box<dyn HandlerT>> {
        let mut bundles = self.bundles.lock().await;

        match bundles.entry(bundle_key.to_string()) {
            Entry::Occupied(e) => {
                Ok(Box::new(WasmHandler::load_by_bundle(e.get()).await?))
            }
            Entry::Vacant(e) => {
                let bundle = self.fetch_bundle(bundle_key).await?;
                let handler = WasmHandler::load_by_bundle(&bundle).await?;
                e.insert(bundle);
                Ok(Box::new(handler))
            }
        }
    }
}

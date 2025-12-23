/// This HandlerManager caches the downloaded bundles, to avoid
/// unnecessary networking.

use crate::handler::HandlerT;
use crate::wasm_handler::WasmHandler;
use std::collections::HashMap;
use race_core::error::{Error, Result};
use std::collections::hash_map::Entry;
use race_core::transport::TransportT;
use race_core::types::GameBundle;

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct HandlerManager {
    bundles: Mutex<HashMap<String, GameBundle>>,
    transport: Arc<dyn TransportT>,
}

impl HandlerManager {
    pub fn new(transport: Arc<dyn TransportT>) -> Self {
        let bundles = Mutex::new(HashMap::default());

        Self {
            bundles,
            transport,
        }
    }

    pub async fn get_handler(&self, bundle_addr: &str) -> Result<Box<dyn HandlerT>> {
        let mut bundles = self.bundles.lock().await;

        match bundles.entry(bundle_addr.to_string()) {
            Entry::Occupied(e) => {
                Ok(Box::new(WasmHandler::load_by_bundle(e.get()).await?))
            }
            Entry::Vacant(e) => {
                let bundle = self.transport
                    .get_game_bundle(bundle_addr)
                    .await?
                    .ok_or(Error::GameBundleNotFound)?;
                let handler = WasmHandler::load_by_bundle(&bundle).await?;
                e.insert(bundle);
                Ok(Box::new(handler))
            }
        }
    }
}

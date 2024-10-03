use async_trait::async_trait;
use race_api::error::Result;

use crate::types::{GetCheckpointParams, SaveCheckpointParams, SaveResult};

#[async_trait]
pub trait StorageT: Send + Sync {
    /// Upload the checkpoint to storage, return the proof.
    async fn save_checkpoint(&self, params: SaveCheckpointParams) -> Result<SaveResult>;

    /// Get data by key from storage.
    async fn get_checkpoint(&self, params: GetCheckpointParams) -> Result<Vec<u8>>;
}

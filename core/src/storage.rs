use async_trait::async_trait;
use crate::error::Result;

use crate::{checkpoint::CheckpointOffChain, types::{GetCheckpointParams, SaveCheckpointParams}};

#[async_trait]
pub trait StorageT: Send + Sync {
    /// Upload the checkpoint to storage, return the proof.
    async fn save_checkpoint(&self, params: SaveCheckpointParams) -> Result<()>;

    /// Get data by key from storage.
    async fn get_checkpoint(&self, params: GetCheckpointParams) -> Result<Option<CheckpointOffChain>>;
}

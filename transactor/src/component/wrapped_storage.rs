use race_core::{checkpoint::CheckpointOffChain, storage::StorageT, types::{GetCheckpointParams, SaveCheckpointParams}};
use race_env::Config;
use jsonrpsee::core::async_trait;
use race_api::error::Result;
use race_local_db::LocalDbStorage;

pub struct WrappedStorage {
    pub(crate) inner: Box<dyn StorageT>,
}

impl WrappedStorage {
    pub async fn try_new(config: &Config) -> Result<Self> {
        let storage = if let Some(ref storage_config) = config.storage {
            LocalDbStorage::try_new(&storage_config.db_file_name)?
        } else {
            LocalDbStorage::try_new_mem()?
        };

        Ok(Self { inner: Box::new(storage) })
    }
}

#[async_trait]
impl StorageT for WrappedStorage {
    async fn save_checkpoint(&self, params: SaveCheckpointParams) -> Result<()> {
        self.inner.save_checkpoint(params).await
    }

    async fn get_checkpoint(&self, params: GetCheckpointParams) -> Result<Option<CheckpointOffChain>> {
        self.inner.get_checkpoint(params).await
    }
}

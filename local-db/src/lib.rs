use std::sync::Arc;

use async_trait::async_trait;
use borsh::BorshDeserialize;
use race_api::error::{Error, Result};
use race_core::{
    checkpoint::CheckpointOffChain,
    storage::StorageT,
    types::{GetCheckpointParams, SaveCheckpointParams},
};
use rusqlite::{params, Connection};
use tokio::sync::Mutex;

pub struct LocalDbStorage {
    conn: Arc<Mutex<Connection>>,
}

#[async_trait]
impl StorageT for LocalDbStorage {
    async fn save_checkpoint(&self, params: SaveCheckpointParams) -> Result<()> {
        let conn = self.conn.lock().await;
        let checkpoint_bs = borsh::to_vec(&params.checkpoint).unwrap();
        conn.execute(
            "INSERT INTO game_checkpoints (game_addr, settle_version, checkpoint) VALUES (?1, ?2, ?3)",
            params![params.game_addr, params.settle_version, checkpoint_bs],
        )
        .map_err(|e| Error::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_checkpoint(&self, params: GetCheckpointParams) -> Result<CheckpointOffChain> {
        let conn = self.conn.lock().await;

        let checkpoint_bs = conn
            .query_row(
                "SELECT checkpoint FROM game_checkpoints WHERE game_addr = ?1 and settle_version = ?2",
                params![params.game_addr, params.settle_version],
                |row| {
                    let checkpoint_bs = row.get::<_, Vec<u8>>(0);
                    Ok(checkpoint_bs)
                }
            )
            .map_err(|e| Error::StorageError(e.to_string()))?
            .map_err(|e| Error::StorageError(e.to_string()))?;

        let checkpoint = CheckpointOffChain::try_from_slice(&checkpoint_bs)
            .map_err(|e| Error::StorageError(e.to_string()))?;

        Ok(checkpoint)
    }
}

pub fn init_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS game_checkpoints (
          game_addr TEXT PRIMARY KEY,
          settle_version INTEGER NOT NULL,
          checkpoint BLOB
        )",
        (),
    )
    .map_err(|e| Error::StorageError(e.to_string()))?;
    Ok(())
}

impl LocalDbStorage {
    pub fn try_new_mem() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| Error::StorageError(e.to_string()))?;

        init_table(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn try_new(db_file_path: &str) -> Result<Self> {
        let conn =
            Connection::open(db_file_path).map_err(|e| Error::StorageError(e.to_string()))?;

        init_table(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[cfg(test)]
mod tests {
    use race_core::checkpoint::Checkpoint;

    use super::*;

    #[tokio::test]
    async fn test_insert_and_query() {
        let game_addr = "testaddr1".to_string();
        let state = vec![1, 2, 3, 4];
        let settle_version = 10;
        // let storage = LocalDbStorage::try_new("trasnactor-db").unwrap();
        let storage = LocalDbStorage::try_new_mem().unwrap();
        let checkpoint = Checkpoint::new(0, 1, 1, &state).derive_offchain_part();
        storage
            .save_checkpoint(SaveCheckpointParams {
                game_addr: game_addr.clone(),
                settle_version,
                checkpoint: checkpoint.clone(),
            })
            .await
            .unwrap();

        let checkpoint_from_db = storage
            .get_checkpoint(GetCheckpointParams {
                game_addr: game_addr.clone(),
                settle_version,
            })
            .await
            .unwrap();

        assert_eq!(checkpoint_from_db, checkpoint);
    }
}

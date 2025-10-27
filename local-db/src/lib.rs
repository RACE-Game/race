use std::sync::Arc;

use async_trait::async_trait;
use borsh::BorshDeserialize;
use race_core::{
    error::{Error, Result},
    checkpoint::CheckpointOffChain,
    storage::StorageT,
    types::{GetCheckpointParams, SaveCheckpointParams},
};
use rusqlite::{params, Connection, OptionalExtension, OpenFlags};
use tokio::sync::Mutex;
use sha256::digest;

pub struct LocalDbStorage {
    conn: Arc<Mutex<Connection>>,
}

#[async_trait]
impl StorageT for LocalDbStorage {
    async fn save_checkpoint(&self, params: SaveCheckpointParams) -> Result<()> {
        let conn = self.conn.lock().await;
        let checkpoint_bs = borsh::to_vec(&params.checkpoint).or(Err(Error::MalformedCheckpoint))?;
        let sha = digest(&checkpoint_bs);
        conn.execute(
            "INSERT OR REPLACE INTO game_checkpoints (game_addr, settle_version, checkpoint, sha) VALUES (?1, ?2, ?3, ?4)",
            params![params.game_addr, params.settle_version, checkpoint_bs, sha],
        )
        .map_err(|e| Error::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_checkpoint(
        &self,
        params: GetCheckpointParams,
    ) -> Result<Option<CheckpointOffChain>> {
        let conn = self.conn.lock().await;

        let checkpoint_bs = conn
            .query_row(
                "SELECT checkpoint FROM game_checkpoints WHERE game_addr = ?1 and settle_version = ?2",
                params![params.game_addr, params.settle_version],
                |row| {
                    row.get::<_, Vec<u8>>(0)
                }
            ).optional()
            .map_err(|e| Error::StorageError(e.to_string()))?;

        if let Some(checkpoint_bs) = checkpoint_bs {
            let checkpoint = CheckpointOffChain::try_from_slice(&checkpoint_bs)
                .map_err(|e| Error::StorageError(e.to_string()))?;
            Ok(Some(checkpoint))
        } else {
            Ok(None)
        }
    }
}

pub fn init_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS game_checkpoints (
          game_addr TEXT NOT NULL,
          settle_version INTEGER NOT NULL,
          checkpoint BLOB NOT NULL,
          sha TEXT NOT NULL,
          PRIMARY KEY(game_addr, settle_version)
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

    pub fn try_new_readonly(db_file_path: &str) -> Result<Self> {
        let conn =
            Connection::open_with_flags(db_file_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| Error::StorageError(e.to_string()))?;

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
    use race_core::context::Versions;
    use race_core::types::GameSpec;

    #[tokio::test]
    async fn test_insert_and_query() {
        let game_addr = "testaddr1".to_string();
        let state = vec![1, 2, 3, 4];
        let settle_version = 10;
        // let storage = LocalDbStorage::try_new("trasnactor-db").unwrap();
        let storage = LocalDbStorage::try_new_mem().unwrap();
        let game_spec = GameSpec {
            game_addr: game_addr.clone(),
            game_id: 0,
            bundle_addr: "".to_string(),
            max_players: 8,
        };

        let checkpoint = Checkpoint::new(0, game_spec, Versions::new(0, settle_version), state).derive_offchain_part();
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

        assert_eq!(checkpoint_from_db, Some(checkpoint));
    }
}

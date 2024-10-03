use std::sync::Arc;

use async_trait::async_trait;
use race_api::error::{Error, Result};
use race_core::{
    storage::StorageT,
    types::{GetCheckpointParams, SaveCheckpointParams, SaveResult},
};
use rusqlite::{params, Connection};
use tokio::sync::Mutex;

pub struct LocalDbStorage {
    conn: Arc<Mutex<Connection>>,
}

#[async_trait]
impl StorageT for LocalDbStorage {
    async fn save_checkpoint(&self, params: SaveCheckpointParams) -> Result<SaveResult> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO game_checkpoints (game_addr, settle_version, state, proof) VALUES (?1, ?2, ?3, ?4)",
            params![params.game_addr, params.settle_version, params.state, params.proof],
        )
        .map_err(|e| Error::StorageError(e.to_string()))?;

        Ok(SaveResult {
            proof: "fake proof".to_string(),
        })
    }

    async fn get_checkpoint(&self, params: GetCheckpointParams) -> Result<Vec<u8>> {
        let conn = self.conn.lock().await;

        let state = conn
            .query_row(
                "SELECT (state) FROM game_checkpoints WHERE game_addr = ?1 and settle_version = ?2",
                params![params.game_addr, params.settle_version],
                |row| Ok(row.get::<_, Vec<u8>>(0)),
            )
            .map_err(|e| Error::StorageError(e.to_string()))?
            .map_err(|e| Error::StorageError(e.to_string()))?;

        Ok(state)
    }
}

pub fn init_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS game_checkpoints (
          id INTEGER PRIMARY KEY,
          game_addr TEXT NOT NULL,
          settle_version INTEGER NOT NULL,
          state BLOB,
          proof TEXT NOT NULL,
          UNIQUE(game_addr, settle_version)
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
        let conn = Connection::open(&db_file_path)
            .map_err(|e| Error::StorageError(e.to_string()))?;

        init_table(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_query() {
        let game_addr = "testaddr1".to_string();
        let state = vec![1, 2, 3, 4];
        let settle_version = 10;
        let storage = LocalDbStorage::try_new("trasnactor-db").unwrap();
        storage
            .save_checkpoint(SaveCheckpointParams {
                game_addr: game_addr.clone(),
                settle_version,
                state: state.clone(),
                proof: "fake proof".to_string(),
            })
            .await
            .unwrap();

        let state_from_db = storage
            .get_checkpoint(GetCheckpointParams {
                game_addr: game_addr.clone(),
                settle_version,
            })
            .await
            .unwrap();

        assert_eq!(state_from_db, state);
    }
}

use sqlx::{Pool, Sqlite};

pub async fn create_tables(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    create_bundles_table(pool).await?;

    Ok(())
}


// create table for bundles, which read wasm file as base64 save into database.
// we may have several bundles, so we need to fetch them when init this program.
async fn create_bundles_table(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    sqlx::query(r#"
create table if not exists game_bundles (
  address text primary key not null,
  data blob not null
)"#).execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    const DB_URL: &str = "sqlite:///Users/miles/workspace/race/facade/data.db";

    async fn create_pool() -> Pool<Sqlite> {
        SqlitePool::connect(DB_URL).await.expect("connect error")
    }

    #[tokio::test]
    async fn test_create() -> anyhow::Result<()> {
        let pool = create_pool().await;
        create_tables(&pool).await?;
        Ok(())
    }
}

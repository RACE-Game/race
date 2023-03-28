use jsonrpsee::core::Error as RpcError;
use race_core::types::GameBundle;
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

type RpcResult<T> = std::result::Result<T, RpcError>;

fn random_addr() -> String {
    Uuid::new_v4().to_string()
}

pub async fn create_game_bundle(pool: &Pool<Sqlite>, bundle: GameBundle) -> anyhow::Result<()> {
    let addr = bundle.addr.clone();
    let data = bundle.data.clone();
    sqlx::query("insert into game_bundles(address, data) values ($1, $2)")
        .bind(addr)
        .bind(data)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_game_bundle_by_addr(pool: &Pool<Sqlite>, addr: &String) -> Option<GameBundle> {
    match sqlx::query("select * from game_bundles where address = ?")
        .bind(addr)
        .fetch_optional(pool)
        .await
        .expect("sql error")
    {
        Some(bundle) => Some(GameBundle {
            addr: bundle.get("address"),
            data: bundle.get("data"),
        }),
        _ => None,
    }

    // GameBundle {

    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Ok;
    use sqlx::SqlitePool;

    async fn create_pool() -> Pool<Sqlite> {
        SqlitePool::connect("sqlite:///Users/miles/workspace/race/facade/data.db")
            .await
            .expect("connect error")
    }

    #[tokio::test]
    async fn test_create_game_bundle() -> anyhow::Result<()> {
        let pool = create_pool().await;

        let bundle = GameBundle {
            addr: random_addr().clone(),
            data: "test".into(),
        };

        create_game_bundle(&pool, bundle).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_game_bundle_by_addr() -> anyhow::Result<()> {
        let pool = create_pool().await;
        let addr = String::from("1ce2456e-0641-4cb4-a197-f2a");
        let result = get_game_bundle_by_addr(&pool, &addr).await;
        println!("{:?}", result);
        Ok(())
    }
}

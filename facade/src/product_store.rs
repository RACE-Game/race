use postgres::{Client, NoTls};

use crate::db::{
    GuestAccount, GuestSession, ProductEventLogEntry, UserProgress, UserRating, UserStats,
};

const DEFAULT_RANK_TIER: &str = "Bronze I";
const DEFAULT_RANK_BUCKET: &str = "Bronze I";
const DEFAULT_LEVEL: i32 = 1;
const DEFAULT_XP: i64 = 0;
const DEFAULT_RATING: i32 = 1000;
const PRODUCT_SCHEMA_V1: &str = include_str!("../sql/product_schema_v1.sql");

pub struct ProductStore {
    client: Client,
}

impl ProductStore {
    pub fn connect(database_url: &str) -> anyhow::Result<Self> {
        let mut client = Client::connect(database_url, NoTls)?;
        Self::prepare_tables(&mut client)?;
        Ok(Self { client })
    }

    fn prepare_tables(client: &mut Client) -> anyhow::Result<()> {
        client.batch_execute(PRODUCT_SCHEMA_V1)?;
        Ok(())
    }

    pub fn create_guest_account(&mut self, guest_account: &GuestAccount) -> anyhow::Result<()> {
        self.client.execute(
            "INSERT INTO guest_account (
                guest_id, player_addr, nick, status, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &guest_account.guest_id,
                &guest_account.player_addr,
                &guest_account.nick,
                &guest_account.status,
                &(guest_account.created_at as i64),
                &(guest_account.updated_at as i64),
            ],
        )?;

        self.initialize_product_state(&guest_account.guest_id, guest_account.created_at)?;
        Ok(())
    }

    fn initialize_product_state(&mut self, guest_id: &str, now: u64) -> anyhow::Result<()> {
        let now = now as i64;
        self.client.execute(
            "INSERT INTO user_progress (guest_id, rank_tier, xp, level, updated_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (guest_id) DO NOTHING",
            &[&guest_id, &DEFAULT_RANK_TIER, &DEFAULT_XP, &DEFAULT_LEVEL, &now],
        )?;
        self.client.execute(
            "INSERT INTO user_rating (guest_id, rating, rank_bucket, updated_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (guest_id) DO NOTHING",
            &[&guest_id, &DEFAULT_RATING, &DEFAULT_RANK_BUCKET, &now],
        )?;
        self.client.execute(
            "INSERT INTO user_stats (guest_id, hands_played, games_played, wins, losses, last_played_at)
             VALUES ($1, 0, 0, 0, 0, NULL)
             ON CONFLICT (guest_id) DO NOTHING",
            &[&guest_id],
        )?;
        Ok(())
    }

    pub fn read_guest_account_by_guest_id(
        &mut self,
        guest_id: &str,
    ) -> anyhow::Result<Option<GuestAccount>> {
        let row = self.client.query_opt(
            "SELECT guest_id, player_addr, nick, status, created_at, updated_at
             FROM guest_account WHERE guest_id = $1",
            &[&guest_id],
        )?;

        Ok(row.map(|row| GuestAccount {
            guest_id: row.get::<_, String>(0),
            player_addr: row.get::<_, String>(1),
            nick: row.get::<_, String>(2),
            status: row.get::<_, String>(3),
            created_at: row.get::<_, i64>(4) as u64,
            updated_at: row.get::<_, i64>(5) as u64,
        }))
    }

    pub fn read_guest_account_by_player_addr(
        &mut self,
        player_addr: &str,
    ) -> anyhow::Result<Option<GuestAccount>> {
        let row = self.client.query_opt(
            "SELECT guest_id, player_addr, nick, status, created_at, updated_at
             FROM guest_account WHERE player_addr = $1",
            &[&player_addr],
        )?;

        Ok(row.map(|row| GuestAccount {
            guest_id: row.get::<_, String>(0),
            player_addr: row.get::<_, String>(1),
            nick: row.get::<_, String>(2),
            status: row.get::<_, String>(3),
            created_at: row.get::<_, i64>(4) as u64,
            updated_at: row.get::<_, i64>(5) as u64,
        }))
    }

    pub fn create_guest_session(&mut self, guest_session: &GuestSession) -> anyhow::Result<()> {
        self.client.execute(
            "INSERT INTO guest_session (
                session_id, guest_id, session_token_hash, created_at, expires_at, revoked_at
             ) VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &guest_session.session_id,
                &guest_session.guest_id,
                &guest_session.session_token_hash,
                &(guest_session.created_at as i64),
                &(guest_session.expires_at as i64),
                &guest_session.revoked_at.map(|v| v as i64),
            ],
        )?;
        Ok(())
    }

    pub fn read_user_progress(&mut self, guest_id: &str) -> anyhow::Result<Option<UserProgress>> {
        let row = self.client.query_opt(
            "SELECT guest_id, rank_tier, xp, level, updated_at
             FROM user_progress WHERE guest_id = $1",
            &[&guest_id],
        )?;

        Ok(row.map(|row| UserProgress {
            guest_id: row.get::<_, String>(0),
            rank_tier: row.get::<_, String>(1),
            xp: row.get::<_, i64>(2) as u64,
            level: row.get::<_, i32>(3) as u32,
            updated_at: row.get::<_, i64>(4) as u64,
        }))
    }

    pub fn read_user_rating(&mut self, guest_id: &str) -> anyhow::Result<Option<UserRating>> {
        let row = self.client.query_opt(
            "SELECT guest_id, rating, rank_bucket, updated_at
             FROM user_rating WHERE guest_id = $1",
            &[&guest_id],
        )?;

        Ok(row.map(|row| UserRating {
            guest_id: row.get::<_, String>(0),
            rating: row.get::<_, i32>(1),
            rank_bucket: row.get::<_, String>(2),
            updated_at: row.get::<_, i64>(3) as u64,
        }))
    }

    pub fn read_user_stats(&mut self, guest_id: &str) -> anyhow::Result<Option<UserStats>> {
        let row = self.client.query_opt(
            "SELECT guest_id, hands_played, games_played, wins, losses, last_played_at
             FROM user_stats WHERE guest_id = $1",
            &[&guest_id],
        )?;

        Ok(row.map(|row| UserStats {
            guest_id: row.get::<_, String>(0),
            hands_played: row.get::<_, i64>(1) as u64,
            games_played: row.get::<_, i64>(2) as u64,
            wins: row.get::<_, i64>(3) as u64,
            losses: row.get::<_, i64>(4) as u64,
            last_played_at: row.get::<_, Option<i64>>(5).map(|v| v as u64),
        }))
    }

    pub fn record_user_joined_game(&mut self, guest_id: &str, now: u64) -> anyhow::Result<()> {
        self.client.execute(
            "UPDATE user_stats
             SET games_played = games_played + 1,
                 last_played_at = $2
             WHERE guest_id = $1",
            &[&guest_id, &(now as i64)],
        )?;
        Ok(())
    }

    pub fn insert_product_event_log_entry(
        &mut self,
        entry: &ProductEventLogEntry,
    ) -> anyhow::Result<bool> {
        let changed = self.client.execute(
            "INSERT INTO product_event_log (
                event_id, event_type, guest_id, created_at
             ) VALUES ($1, $2, $3, $4)
             ON CONFLICT (event_id) DO NOTHING",
            &[
                &entry.event_id,
                &entry.event_type,
                &entry.guest_id,
                &(entry.created_at as i64),
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn update_user_progress(&mut self, progress: &UserProgress) -> anyhow::Result<()> {
        self.client.execute(
            "UPDATE user_progress
             SET rank_tier = $2,
                 xp = $3,
                 level = $4,
                 updated_at = $5
             WHERE guest_id = $1",
            &[
                &progress.guest_id,
                &progress.rank_tier,
                &(progress.xp as i64),
                &(progress.level as i32),
                &(progress.updated_at as i64),
            ],
        )?;
        Ok(())
    }

    pub fn increment_user_hands_played(&mut self, guest_id: &str) -> anyhow::Result<()> {
        self.client.execute(
            "UPDATE user_stats
             SET hands_played = hands_played + 1
             WHERE guest_id = $1",
            &[&guest_id],
        )?;
        Ok(())
    }

    pub fn read_guest_session_by_token_hash(
        &mut self,
        session_token_hash: &str,
    ) -> anyhow::Result<Option<GuestSession>> {
        let row = self.client.query_opt(
            "SELECT session_id, guest_id, session_token_hash, created_at, expires_at, revoked_at
             FROM guest_session WHERE session_token_hash = $1",
            &[&session_token_hash],
        )?;

        Ok(row.map(|row| GuestSession {
            session_id: row.get::<_, String>(0),
            guest_id: row.get::<_, String>(1),
            session_token_hash: row.get::<_, String>(2),
            created_at: row.get::<_, i64>(3) as u64,
            expires_at: row.get::<_, i64>(4) as u64,
            revoked_at: row.get::<_, Option<i64>>(5).map(|v| v as u64),
        }))
    }

    pub fn revoke_guest_session(
        &mut self,
        session_token_hash: &str,
        revoked_at: u64,
    ) -> anyhow::Result<()> {
        self.client.execute(
            "UPDATE guest_session
             SET revoked_at = $1
             WHERE session_token_hash = $2 AND revoked_at IS NULL",
            &[&(revoked_at as i64), &session_token_hash],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ProductStore;
    use crate::db::{GuestAccount, GuestSession};
    use postgres::{Client, NoTls};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_pg_admin_url() -> Option<String> {
        std::env::var("RACE_FACADE_TEST_POSTGRES_URL").ok()
    }

    fn db_url_for_database(admin_url: &str, db_name: &str) -> String {
        if admin_url.contains("://") {
            match admin_url.rsplit_once('/') {
                Some((prefix, _)) => format!("{prefix}/{db_name}"),
                None => admin_url.to_string(),
            }
        } else if admin_url.contains("dbname=") {
            admin_url.replacen("dbname=postgres", &format!("dbname={db_name}"), 1)
        } else {
            format!("{admin_url} dbname={db_name}")
        }
    }

    #[test]
    fn test_product_store_guest_flow_if_configured() -> anyhow::Result<()> {
        let Some(admin_url) = test_pg_admin_url() else {
            return Ok(());
        };

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos();
        let db_name = format!("race_facade_test_{unique}");

        {
            let mut admin = Client::connect(&admin_url, NoTls)?;
            admin.batch_execute(&format!("CREATE DATABASE {db_name}"))?;
        }

        let product_db_url = db_url_for_database(&admin_url, &db_name);
        let mut store = ProductStore::connect(&product_db_url)?;

        let guest_account = GuestAccount {
            guest_id: "guest-pg-1".into(),
            player_addr: "guest_player_pg_1".into(),
            nick: "PgGuest".into(),
            status: "active".into(),
            created_at: 100,
            updated_at: 100,
        };

        let guest_session = GuestSession {
            session_id: "guest-session-pg-1".into(),
            guest_id: "guest-pg-1".into(),
            session_token_hash: "pg-token-hash".into(),
            created_at: 100,
            expires_at: 200,
            revoked_at: None,
        };

        store.create_guest_account(&guest_account)?;
        store.create_guest_session(&guest_session)?;

        let stored_account = store
            .read_guest_account_by_guest_id("guest-pg-1")?
            .expect("guest account");
        assert_eq!(stored_account.player_addr, "guest_player_pg_1");

        let stored_session = store
            .read_guest_session_by_token_hash("pg-token-hash")?
            .expect("guest session");
        assert_eq!(stored_session.guest_id, "guest-pg-1");

        store.revoke_guest_session("pg-token-hash", 150)?;
        let revoked_session = store
            .read_guest_session_by_token_hash("pg-token-hash")?
            .expect("revoked session");
        assert_eq!(revoked_session.revoked_at, Some(150));

        drop(store);

        {
            let mut admin = Client::connect(&admin_url, NoTls)?;
            admin.batch_execute(&format!("DROP DATABASE IF EXISTS {db_name} WITH (FORCE)"))?;
        }

        Ok(())
    }
}

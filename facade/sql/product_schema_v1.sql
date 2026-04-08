CREATE TABLE IF NOT EXISTS guest_account (
    guest_id TEXT PRIMARY KEY,
    player_addr TEXT NOT NULL UNIQUE,
    nick TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS guest_session (
    session_id TEXT PRIMARY KEY,
    guest_id TEXT NOT NULL REFERENCES guest_account(guest_id),
    session_token_hash TEXT NOT NULL UNIQUE,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    revoked_at BIGINT
);

CREATE TABLE IF NOT EXISTS user_progress (
    guest_id TEXT PRIMARY KEY REFERENCES guest_account(guest_id),
    rank_tier TEXT NOT NULL,
    xp BIGINT NOT NULL,
    level INTEGER NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_rating (
    guest_id TEXT PRIMARY KEY REFERENCES guest_account(guest_id),
    rating INTEGER NOT NULL,
    rank_bucket TEXT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_stats (
    guest_id TEXT PRIMARY KEY REFERENCES guest_account(guest_id),
    hands_played BIGINT NOT NULL DEFAULT 0,
    games_played BIGINT NOT NULL DEFAULT 0,
    wins BIGINT NOT NULL DEFAULT 0,
    losses BIGINT NOT NULL DEFAULT 0,
    last_played_at BIGINT
);

CREATE TABLE IF NOT EXISTS product_event_log (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    guest_id TEXT NOT NULL REFERENCES guest_account(guest_id),
    created_at BIGINT NOT NULL
);

DROP TABLE IF EXISTS fetch_diagnostics;
DROP TABLE IF EXISTS user_cache_snapshots;

CREATE TABLE users_v2 (
    username TEXT PRIMARY KEY,
    links TEXT NOT NULL DEFAULT '[]',
    rank INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

INSERT INTO users_v2 (
    username,
    links,
    rank,
    created_at
)
SELECT
    username,
    links,
    rank,
    created_at
FROM users;

DROP TABLE users;

ALTER TABLE users_v2
    RENAME TO users;

CREATE INDEX idx_users_rank
    ON users(rank);

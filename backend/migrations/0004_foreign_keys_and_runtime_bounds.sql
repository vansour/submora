CREATE TABLE fetch_diagnostics_v2 (
    username TEXT NOT NULL,
    source_url TEXT NOT NULL,
    status TEXT NOT NULL,
    detail TEXT,
    http_status INTEGER,
    content_type TEXT,
    body_bytes INTEGER,
    redirect_count INTEGER NOT NULL DEFAULT 0,
    is_html INTEGER NOT NULL DEFAULT 0,
    fetched_at INTEGER DEFAULT (strftime('%s', 'now')),
    PRIMARY KEY (username, source_url),
    FOREIGN KEY (username) REFERENCES users(username) ON DELETE CASCADE
);

INSERT INTO fetch_diagnostics_v2 (
    username,
    source_url,
    status,
    detail,
    http_status,
    content_type,
    body_bytes,
    redirect_count,
    is_html,
    fetched_at
)
SELECT
    diagnostics.username,
    diagnostics.source_url,
    diagnostics.status,
    diagnostics.detail,
    diagnostics.http_status,
    diagnostics.content_type,
    diagnostics.body_bytes,
    diagnostics.redirect_count,
    diagnostics.is_html,
    diagnostics.fetched_at
FROM fetch_diagnostics AS diagnostics
WHERE EXISTS (
    SELECT 1
    FROM users
    WHERE users.username = diagnostics.username
);

DROP TABLE fetch_diagnostics;

ALTER TABLE fetch_diagnostics_v2
    RENAME TO fetch_diagnostics;

CREATE INDEX idx_fetch_diagnostics_username
    ON fetch_diagnostics(username);

CREATE TABLE user_cache_snapshots_v2 (
    username TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    line_count INTEGER NOT NULL DEFAULT 0,
    body_bytes INTEGER NOT NULL DEFAULT 0,
    generated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    expires_at INTEGER NOT NULL,
    source_config_version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (username) REFERENCES users(username) ON DELETE CASCADE
);

INSERT INTO user_cache_snapshots_v2 (
    username,
    content,
    line_count,
    body_bytes,
    generated_at,
    expires_at,
    source_config_version
)
SELECT
    snapshots.username,
    snapshots.content,
    snapshots.line_count,
    snapshots.body_bytes,
    snapshots.generated_at,
    snapshots.expires_at,
    snapshots.source_config_version
FROM user_cache_snapshots AS snapshots
WHERE EXISTS (
    SELECT 1
    FROM users
    WHERE users.username = snapshots.username
);

DROP TABLE user_cache_snapshots;

ALTER TABLE user_cache_snapshots_v2
    RENAME TO user_cache_snapshots;

CREATE INDEX idx_user_cache_snapshots_expires_at
    ON user_cache_snapshots(expires_at);

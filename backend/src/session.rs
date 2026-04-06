use std::time::Duration as StdDuration;

use sqlx::SqlitePool;
use time::Duration;
use tokio::task::JoinHandle;
use tower_sessions::{
    Expiry, SessionManagerLayer, cookie::SameSite, session_store::ExpiredDeletion,
};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::warn;

use crate::config::ServerConfig;

pub async fn build_session_store(pool: SqlitePool) -> Result<SqliteStore, sqlx::Error> {
    let store = SqliteStore::new(pool);
    store.migrate().await?;
    Ok(store)
}

pub fn build_session_layer(
    store: SqliteStore,
    config: &ServerConfig,
) -> SessionManagerLayer<SqliteStore> {
    SessionManagerLayer::new(store)
        .with_secure(config.cookie_secure)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(
            config.session_ttl_minutes,
        )))
}

pub fn spawn_expired_session_cleanup(
    store: SqliteStore,
    cleanup_interval_secs: u64,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let interval = StdDuration::from_secs(cleanup_interval_secs);
        loop {
            // clone store 因为 continuously_delete_expired 会消耗 self
            let store_clone = store.clone();
            match store_clone.continuously_delete_expired(interval).await {
                Ok(_) => {
                    // 正常情况下不应该返回，如果返回说明任务意外退出
                    warn!("session cleanup task exited unexpectedly, restarting in 10s");
                }
                Err(error) => {
                    warn!(error = %error, "session cleanup task failed, restarting in 10s");
                }
            }
            // 等待 10 秒后重启
            tokio::time::sleep(StdDuration::from_secs(10)).await;
        }
    })
}

use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Semaphore;

use crate::{
    config::ServerConfig,
    security::{LoginRateLimiter, PublicRateLimiter},
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: SqlitePool,
    pub fetch_semaphore: Arc<Semaphore>,
    pub login_rate_limiter: LoginRateLimiter,
    pub public_rate_limiter: PublicRateLimiter,
    pub config: ServerConfig,
}

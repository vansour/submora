use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, HeaderValue, Response, header},
    response::{IntoResponse, Json},
};
use sqlx::Row;
use tracing::info;

use crate::{core, error::ApiError, shared::api::AppInfoResponse, state::AppState, subscriptions};

pub async fn healthz() -> &'static str {
    "ok"
}

pub async fn app_info(State(state): State<Arc<AppState>>) -> Json<AppInfoResponse> {
    Json(AppInfoResponse {
        name: core::APP_NAME.to_string(),
        phase: core::CURRENT_PHASE,
        frontend: "vue3-vite".to_string(),
        backend: "axum-0.8.8".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        web_dist_dir: state.config.web_dist_dir.display().to_string(),
    })
}

pub async fn merged_user(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer_addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let client_ip = crate::security::request_client_ip(
        &headers,
        Some(peer_addr),
        state.config.trust_proxy_headers,
    )
    .map(|ip| ip.to_string())
    .unwrap_or_else(|| "unknown".to_string());
    let rate_limit_key = format!("{client_ip}:{}", username.trim().to_ascii_lowercase());
    state
        .public_rate_limiter
        .check_and_record(&rate_limit_key)
        .await?;

    let Some(links) = load_public_user_links(&state, &username).await? else {
        return Err(ApiError::not_found("user not found"));
    };

    if links.is_empty() {
        info!(username, link_count = 0, "served empty public merged feed");
        return Ok(text_response(String::new()));
    }

    let merged = subscriptions::fetch_and_merge_for_user(
        subscriptions::FetchRuntime {
            fetch_timeout_secs: state.config.fetch_timeout_secs,
            fetch_host_overrides: &state.config.fetch_host_overrides,
            semaphore: state.fetch_semaphore.clone(),
            concurrent_limit: state.config.concurrent_limit,
        },
        links.clone(),
    )
    .await;

    info!(
        username,
        link_count = links.len(),
        "served live public merged feed"
    );
    Ok(text_response(merged))
}

fn text_response(body: String) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    response
}

async fn load_public_user_links(
    state: &AppState,
    username: &str,
) -> Result<Option<Vec<String>>, ApiError> {
    let row = sqlx::query("SELECT links FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.db)
        .await?;

    row.map(links_from_row).transpose()
}

fn links_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Vec<String>, ApiError> {
    let value: serde_json::Value = row.get("links");
    serde_json::from_value(value)
        .map_err(|error| ApiError::internal(format!("failed to decode stored links: {error}")))
}

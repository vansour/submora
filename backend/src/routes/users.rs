use std::{collections::HashSet, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use futures::stream::StreamExt;
use sqlx::Row;
use tower_sessions::Session;
use tracing::info;

use crate::{
    cache,
    core::{is_valid_username, normalize_links_preserve_order},
    diagnostics,
    error::{ApiError, ApiResult, message_response},
    security,
    shared::{
        api::ApiMessage,
        users::{
            CreateUserRequest, LinksPayload, UserCacheStatusResponse, UserDiagnosticsResponse,
            UserLinksResponse, UserOrderPayload, UserSummary,
        },
    },
    state::AppState,
    subscriptions,
};

const SESSION_KEY: &str = "user_id";

struct UserLinkConfig {
    links: Vec<String>,
    config_version: i64,
}

async fn require_auth(session: &Session) -> ApiResult<String> {
    let Some(username) = session.get::<String>(SESSION_KEY).await? else {
        return Err(ApiError::unauthorized());
    };

    Ok(username)
}

async fn load_user_link_config(state: &AppState, username: &str) -> ApiResult<UserLinkConfig> {
    let row = sqlx::query("SELECT links, config_version FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Err(ApiError::not_found("user not found"));
    };

    let value: serde_json::Value = row.get("links");
    let links = serde_json::from_value(value)
        .map_err(|error| ApiError::internal(format!("failed to decode stored links: {error}")))?;

    Ok(UserLinkConfig {
        links,
        config_version: row.get("config_version"),
    })
}

async fn ensure_user_exists(state: &AppState, username: &str) -> ApiResult<()> {
    let row = sqlx::query("SELECT 1 FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.db)
        .await?;

    if row.is_some() {
        Ok(())
    } else {
        Err(ApiError::not_found("user not found"))
    }
}

async fn validate_links_safely(state: &AppState, links: &[String]) -> ApiResult<()> {
    let concurrency = state.config.concurrent_limit.min(links.len()).max(1);
    let resolver = state.dns_resolver.clone();
    let mut validations = futures::stream::iter(links.to_vec())
        .map(|link| {
            let resolver = resolver.clone();
            async move {
                subscriptions::validate_safe_url(&resolver, &link)
                    .await
                    .map_err(|message| ApiError::validation("links", message))
            }
        })
        .buffer_unordered(concurrency);

    while let Some(result) = validations.next().await {
        result?;
    }

    Ok(())
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> ApiResult<Json<Vec<UserSummary>>> {
    let _ = require_auth(&session).await?;

    let rows = sqlx::query("SELECT username FROM users ORDER BY rank ASC, username ASC")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(
        rows.into_iter()
            .map(|row| UserSummary {
                username: row.get("username"),
            })
            .collect(),
    ))
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    session: Session,
    Json(payload): Json<CreateUserRequest>,
) -> ApiResult<Json<UserSummary>> {
    let actor = require_auth(&session).await?;
    security::verify_csrf(&session, &headers).await?;
    let username = payload.username.trim();

    if !is_valid_username(username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    let mut tx = state.db.begin().await?;
    let user_count: i64 = sqlx::query("SELECT COUNT(*) FROM users")
        .fetch_one(&mut *tx)
        .await?
        .get(0);
    if user_count as usize >= state.config.max_users {
        return Err(ApiError::validation(
            "username",
            format!("maximum {} users allowed", state.config.max_users),
        ));
    }

    let next_rank: i64 = sqlx::query("SELECT COALESCE(MAX(rank), -1) + 1 FROM users")
        .fetch_one(&mut *tx)
        .await?
        .get(0);

    let result = sqlx::query("INSERT INTO users (username, links, rank) VALUES ($1, '[]', $2)")
        .bind(username)
        .bind(next_rank)
        .execute(&mut *tx)
        .await;

    match result {
        Ok(_) => {
            tx.commit().await?;
            info!(actor, username, "created subscription user");
            Ok(Json(UserSummary {
                username: username.to_string(),
            }))
        }
        Err(sqlx::Error::Database(db_error)) if db_error.message().contains("UNIQUE") => {
            Err(ApiError::validation("username", "username already exists"))
        }
        Err(error) => Err(ApiError::from(error)),
    }
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    session: Session,
    Path(username): Path<String>,
) -> ApiResult<Json<ApiMessage>> {
    let actor = require_auth(&session).await?;
    security::verify_csrf(&session, &headers).await?;

    if !is_valid_username(&username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    let mut tx = state.db.begin().await?;
    let result = sqlx::query("DELETE FROM users WHERE username = $1")
        .bind(&username)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }

    tx.commit().await?;
    info!(actor, username, "deleted subscription user");
    Ok(message_response("deleted"))
}

pub async fn get_links(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(username): Path<String>,
) -> ApiResult<Json<UserLinksResponse>> {
    let _ = require_auth(&session).await?;

    if !is_valid_username(&username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    let config = load_user_link_config(&state, &username).await?;
    Ok(Json(UserLinksResponse {
        username,
        links: config.links,
    }))
}

pub async fn set_links(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    session: Session,
    Path(username): Path<String>,
    Json(payload): Json<LinksPayload>,
) -> ApiResult<Json<UserLinksResponse>> {
    let actor = require_auth(&session).await?;
    security::verify_csrf(&session, &headers).await?;

    if !is_valid_username(&username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    let links = normalize_links_preserve_order(&payload.links, state.config.max_links_per_user)
        .map_err(|message| ApiError::validation("links", message))?;

    validate_links_safely(&state, &links).await?;

    let value = serde_json::to_value(&links)
        .map_err(|error| ApiError::internal(format!("failed to encode links: {error}")))?;

    let mut tx = state.db.begin().await?;
    let result = sqlx::query(
        "UPDATE users SET links = $1, config_version = config_version + 1 WHERE username = $2",
    )
    .bind(value)
    .bind(&username)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }

    sqlx::query("DELETE FROM user_cache_snapshots WHERE username = $1")
        .bind(&username)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM fetch_diagnostics WHERE username = $1")
        .bind(&username)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    info!(
        actor,
        username,
        link_count = links.len(),
        "updated user links"
    );
    Ok(Json(UserLinksResponse { username, links }))
}

pub async fn set_order(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    session: Session,
    Json(payload): Json<UserOrderPayload>,
) -> ApiResult<Json<Vec<String>>> {
    let actor = require_auth(&session).await?;
    security::verify_csrf(&session, &headers).await?;

    if payload.order.is_empty() {
        return Err(ApiError::validation("order", "must not be empty"));
    }
    if payload.order.len() > state.config.max_users {
        return Err(ApiError::validation(
            "order",
            format!("maximum {} users allowed", state.config.max_users),
        ));
    }

    let mut seen = HashSet::new();
    for username in &payload.order {
        if !is_valid_username(username) {
            return Err(ApiError::validation(
                "order",
                format!("invalid username: {username}"),
            ));
        }
        if !seen.insert(username.clone()) {
            return Err(ApiError::validation(
                "order",
                format!("duplicate username: {username}"),
            ));
        }
    }

    let existing_rows = sqlx::query("SELECT username FROM users ORDER BY rank ASC")
        .fetch_all(&state.db)
        .await?;
    let existing: HashSet<String> = existing_rows
        .into_iter()
        .map(|row| row.get("username"))
        .collect();
    if existing != seen {
        return Err(ApiError::validation(
            "order",
            "order must include every existing user exactly once",
        ));
    }

    let mut tx = state.db.begin().await?;
    for (index, username) in payload.order.iter().enumerate() {
        sqlx::query("UPDATE users SET rank = $1 WHERE username = $2")
            .bind(index as i64)
            .bind(username)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    info!(
        actor,
        user_count = payload.order.len(),
        "updated user order"
    );

    Ok(Json(payload.order))
}

pub async fn get_diagnostics(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(username): Path<String>,
) -> ApiResult<Json<UserDiagnosticsResponse>> {
    let _ = require_auth(&session).await?;

    if !is_valid_username(&username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    let config = load_user_link_config(&state, &username).await?;
    let diagnostics =
        diagnostics::load_user_diagnostics(&state.db, &username, &config.links).await?;
    Ok(Json(diagnostics))
}

pub async fn get_cache_status(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(username): Path<String>,
) -> ApiResult<Json<UserCacheStatusResponse>> {
    let _ = require_auth(&session).await?;

    if !is_valid_username(&username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    let config = load_user_link_config(&state, &username).await?;
    let cache_status =
        cache::load_user_cache_status(&state.db, &username, config.config_version).await?;
    Ok(Json(cache_status))
}

pub async fn refresh_cache(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    session: Session,
    Path(username): Path<String>,
) -> ApiResult<Json<UserCacheStatusResponse>> {
    let actor = require_auth(&session).await?;
    security::verify_csrf(&session, &headers).await?;

    if !is_valid_username(&username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    let config = load_user_link_config(&state, &username).await?;
    if config.links.is_empty() {
        cache::clear_user_snapshot(&state.db, &username).await?;
        info!(
            actor,
            username,
            cache_state = "empty",
            "refreshed user cache"
        );
        return Ok(Json(cache::empty_status(&username)));
    }

    match cache::rebuild_user_snapshot(&state, &username, config.links, config.config_version)
        .await?
    {
        Some(snapshot) => {
            let status = cache::status_from_snapshot(&username, Some(&snapshot));
            info!(
                actor,
                username,
                cache_state = status.state.as_str(),
                "refreshed user cache"
            );
            Ok(Json(status))
        }
        None => {
            let status =
                cache::load_user_cache_status(&state.db, &username, config.config_version).await?;
            info!(
                actor,
                username,
                cache_state = status.state.as_str(),
                "refreshed user cache"
            );
            Ok(Json(status))
        }
    }
}

pub async fn clear_cache(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    session: Session,
    Path(username): Path<String>,
) -> ApiResult<Json<ApiMessage>> {
    let actor = require_auth(&session).await?;
    security::verify_csrf(&session, &headers).await?;

    if !is_valid_username(&username) {
        return Err(ApiError::validation("username", "invalid username"));
    }

    ensure_user_exists(&state, &username).await?;
    cache::clear_user_snapshot(&state.db, &username).await?;
    info!(actor, username, "cleared user cache");
    Ok(message_response("cache cleared"))
}

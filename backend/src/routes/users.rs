use std::{collections::HashSet, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use sqlx::Row;
use tower_sessions::Session;
use tracing::info;

use crate::{
    core::{is_valid_username, normalize_links_preserve_order},
    error::{ApiError, ApiResult, message_response},
    security,
    shared::{
        api::ApiMessage,
        users::{
            CreateUserRequest, LinksPayload, UserLinksResponse, UserOrderPayload, UserSummary,
        },
    },
    state::AppState,
};

const SESSION_KEY: &str = "user_id";

async fn require_auth(session: &Session) -> ApiResult<String> {
    let Some(username) = session.get::<String>(SESSION_KEY).await? else {
        return Err(ApiError::unauthorized());
    };

    Ok(username)
}

async fn load_user_links(state: &AppState, username: &str) -> ApiResult<Vec<String>> {
    let row = sqlx::query("SELECT links FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Err(ApiError::not_found("user not found"));
    };

    let value: serde_json::Value = row.get("links");
    serde_json::from_value(value)
        .map_err(|error| ApiError::internal(format!("failed to decode stored links: {error}")))
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

    let links = load_user_links(&state, &username).await?;
    Ok(Json(UserLinksResponse { username, links }))
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

    let value = serde_json::to_value(&links)
        .map_err(|error| ApiError::internal(format!("failed to encode links: {error}")))?;

    let result = sqlx::query("UPDATE users SET links = $1 WHERE username = $2")
        .bind(value)
        .bind(&username)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }

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

use std::{net::SocketAddr, sync::Arc};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::HeaderMap,
};
use sqlx::Row;
use tower_sessions::Session;
use tracing::info;

use crate::{
    core::{is_valid_password_length, is_valid_username},
    error::{ApiError, ApiResult, message_response},
    security,
    shared::{
        api::ApiMessage,
        auth::{CsrfTokenResponse, CurrentUserResponse, LoginRequest},
    },
    state::AppState,
};

const SESSION_KEY: &str = "user_id";

fn validate_login(payload: &LoginRequest) -> ApiResult<()> {
    if !is_valid_username(payload.username.trim()) {
        return Err(ApiError::validation("username", "invalid username"));
    }
    if !is_valid_password_length(&payload.password) {
        return Err(ApiError::validation(
            "password",
            "password must be 1-128 characters",
        ));
    }
    Ok(())
}

pub async fn csrf_token(session: Session) -> ApiResult<Json<CsrfTokenResponse>> {
    security::csrf_token(session).await
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    session: Session,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Json<ApiMessage>> {
    validate_login(&payload)?;
    security::verify_csrf(&session, &headers).await?;

    let login_key = security::login_rate_limit_key(
        &headers,
        payload.username.trim(),
        Some(peer_addr),
        state.config.trust_proxy_headers,
    );
    state.login_rate_limiter.check(&login_key).await?;

    let row = sqlx::query("SELECT password_hash FROM admins WHERE username = $1")
        .bind(payload.username.trim())
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        state.login_rate_limiter.record_failure(&login_key).await;
        return Err(ApiError::unauthorized());
    };

    let hash: String = row.get("password_hash");
    let parsed_hash = PasswordHash::new(&hash)
        .map_err(|_| ApiError::internal("invalid password hash in database"))?;

    if Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        state.login_rate_limiter.record_failure(&login_key).await;
        return Err(ApiError::unauthorized());
    }

    state.login_rate_limiter.record_success(&login_key).await;
    session.cycle_id().await?;
    session.clear().await;
    session
        .insert(SESSION_KEY, payload.username.trim().to_string())
        .await?;
    security::rotate_csrf_token(&session).await?;
    info!(username = payload.username.trim(), "admin login succeeded");
    Ok(message_response("Logged in"))
}

pub async fn logout(headers: HeaderMap, session: Session) -> ApiResult<Json<ApiMessage>> {
    security::verify_csrf(&session, &headers).await?;
    let current_user = session.get::<String>(SESSION_KEY).await?;
    session.flush().await?;
    if let Some(username) = current_user {
        info!(username, "admin logout succeeded");
    }
    Ok(message_response("Logged out"))
}

pub async fn me(session: Session) -> ApiResult<Json<CurrentUserResponse>> {
    let Some(username) = session.get::<String>(SESSION_KEY).await? else {
        return Err(ApiError::unauthorized());
    };

    Ok(Json(CurrentUserResponse { username }))
}

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::shared::api::{ApiErrorBody, ApiMessage};

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    retry_after_secs: Option<u64>,
}

impl ApiError {
    pub fn validation(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "validation",
            message: format!("{field}: {}", message.into()),
            retry_after_secs: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: "Please login".to_string(),
            retry_after_secs: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
            retry_after_secs: None,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: message.into(),
            retry_after_secs: None,
        }
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "too_many_requests",
            message: message.into(),
            retry_after_secs: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal",
            message: message.into(),
            retry_after_secs: None,
        }
    }

    pub fn with_retry_after(mut self, retry_after_secs: u64) -> Self {
        self.retry_after_secs = Some(retry_after_secs.max(1));
        self
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::internal(format!("database error: {error}"))
    }
}

impl From<tower_sessions::session::Error> for ApiError {
    fn from(error: tower_sessions::session::Error) -> Self {
        Self::internal(format!("session error: {error}"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let Self {
            status,
            code,
            message,
            retry_after_secs,
        } = self;

        let mut response = (
            status,
            Json(ApiErrorBody {
                error: code.to_string(),
                message,
            }),
        )
            .into_response();

        if let Some(retry_after_secs) = retry_after_secs
            && let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }

        response
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub fn message_response(message: impl Into<String>) -> Json<ApiMessage> {
    Json(ApiMessage {
        message: message.into(),
    })
}

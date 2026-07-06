use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Application error type. Every handler returns `Result<_, AppError>`.
///
/// `NotFound`/`Conflict` carry an `action` hint (`"register"` / `"login"`) so the
/// frontend can transparently switch ceremonies. Internal errors are logged and
/// collapsed to a generic 500 so no implementation details leak to clients.
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("not found")]
    NotFound(&'static str),

    #[error("conflict")]
    Conflict(&'static str),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error(transparent)]
    Webauthn(#[from] webauthn_rs::prelude::WebauthnError),

    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    Session(#[from] tower_sessions::session::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({ "error": "unauthorized" }))
            }
            AppError::NotFound(action) => (
                StatusCode::NOT_FOUND,
                json!({ "error": "not found", "action": action }),
            ),
            AppError::Conflict(action) => (
                StatusCode::CONFLICT,
                json!({ "error": "conflict", "action": action }),
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, json!({ "error": msg })),
            AppError::Webauthn(e) => {
                tracing::warn!(error = %e, "webauthn ceremony failed");
                (
                    StatusCode::UNAUTHORIZED,
                    json!({ "error": "authentication failed" }),
                )
            }
            AppError::Db(e) => {
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "internal error" }),
                )
            }
            AppError::Session(e) => {
                tracing::error!(error = %e, "session error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "internal error" }),
                )
            }
            AppError::Other(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "internal error" }),
                )
            }
        };
        (status, Json(body)).into_response()
    }
}

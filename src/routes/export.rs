use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use subtle::ConstantTimeEq;

use crate::db;
use crate::error::AppError;
use crate::models::Registration;
use crate::state::AppState;

/// `GET /api/export` — returns all signups as JSON. Guarded by a bearer token
/// compared in constant time. No session involved.
pub async fn export(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Registration>>, AppError> {
    let provided = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");

    let ok: bool = provided
        .as_bytes()
        .ct_eq(state.export_token.as_bytes())
        .into();
    if !ok {
        return Err(AppError::Unauthorized);
    }

    let registrations = db::all_registrations(&state.db).await?;
    Ok(Json(registrations))
}

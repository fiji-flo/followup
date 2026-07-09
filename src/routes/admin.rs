//! Admin endpoint(s), guarded by a separate bearer token (`ADMIN_TOKEN`, distinct
//! from the export token) compared in constant time. No session involved.

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use serde_json::{Value, json};
use subtle::ConstantTimeEq;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

fn check_admin_token(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let provided = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");

    let ok: bool = provided
        .as_bytes()
        .ct_eq(state.admin_token.as_bytes())
        .into();
    if !ok {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

/// `POST /api/admin/phase2/activate` — flips the app into phase 2, unlocking login
/// and the contact-info form for everyone who registered a passkey in phase 1.
/// Idempotent.
pub async fn activate_phase2(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    check_admin_token(&state, &headers)?;
    db::set_phase2_active(&state.db, true).await?;
    Ok(Json(json!({ "phase2_active": true })))
}

use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};
use tower_sessions::Session;

use crate::db;
use crate::error::AppError;
use crate::models::SignupRequest;
use crate::session::{AUTHED_USER, AuthedUser};
use crate::state::AppState;

/// `POST /api/signup` — requires an authenticated session (a completed WebAuthn
/// ceremony). The email is read from the session, never trusted from the body.
pub async fn submit(
    State(state): State<AppState>,
    session: Session,
    Json(req): Json<SignupRequest>,
) -> Result<Json<Value>, AppError> {
    let authed: AuthedUser = session
        .get(AUTHED_USER)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !req.gdpr_consent {
        return Err(AppError::BadRequest("consent is required".into()));
    }

    for (name, value) in [
        ("full_name", &req.full_name),
        ("company", &req.company),
        ("street", &req.street),
        ("postal_code", &req.postal_code),
        ("city", &req.city),
        ("country", &req.country),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::BadRequest(format!("{name} is required")));
        }
    }

    let email = db::email_for_user(&state.db, authed.user_id)
        .await?
        .unwrap_or(authed.email);

    db::upsert_signup(&state.db, authed.user_id, &email, &req).await?;
    Ok(Json(json!({ "ok": true })))
}

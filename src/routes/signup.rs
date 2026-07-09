use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};
use tower_sessions::Session;

use crate::db;
use crate::error::AppError;
use crate::models::SignupRequest;
use crate::session::{AUTHED_USER, AuthedUser};
use crate::state::AppState;

/// `GET /api/signup` — the current user's existing signup (for pre-filling the edit
/// form) plus their key-bound email. `signup` is `null` if they haven't submitted yet.
/// Requires an authenticated session, and only unlocks once phase 2 is active.
pub async fn current(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Value>, AppError> {
    let authed: AuthedUser = session
        .get(AUTHED_USER)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !db::phase2_active(&state.db).await? {
        return Err(AppError::PhaseLocked);
    }
    let signup = db::get_signup(&state.db, authed.user_id).await?;
    Ok(Json(json!({ "email": authed.email, "signup": signup })))
}

/// `POST /api/signup` — requires an authenticated session (a completed WebAuthn
/// ceremony) and phase 2 to be active. The email is read from the session, never
/// trusted from the body. Upserts, so re-submitting edits the existing signup
/// instead of creating a duplicate.
pub async fn submit(
    State(state): State<AppState>,
    session: Session,
    Json(req): Json<SignupRequest>,
) -> Result<Json<Value>, AppError> {
    let authed: AuthedUser = session
        .get(AUTHED_USER)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !db::phase2_active(&state.db).await? {
        return Err(AppError::PhaseLocked);
    }

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

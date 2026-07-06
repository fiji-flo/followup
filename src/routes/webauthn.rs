//! The four WebAuthn ceremony endpoints.
//!
//! The frontend posts a single email. A brand-new email registers a passkey; a
//! known email authenticates. Branching is client-driven via the `action` hint
//! returned on `409` (register -> "login") and `404` (login -> "register").

use axum::Json;
use axum::extract::State;
use tower_sessions::Session;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse,
};

use crate::db;
use crate::error::AppError;
use crate::models::{AuthenticatedResponse, EmailRequest};
use crate::session::{AUTH_STATE, AUTHED_USER, AuthState, AuthedUser, REG_STATE, RegState};
use crate::state::AppState;

fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

fn validate_email(email: &str) -> Result<(), AppError> {
    if email.len() < 3 || !email.contains('@') || email.len() > 254 {
        return Err(AppError::BadRequest("invalid email address".into()));
    }
    Ok(())
}

/// `POST /api/register/start`
pub async fn register_start(
    State(state): State<AppState>,
    session: Session,
    Json(req): Json<EmailRequest>,
) -> Result<Json<CreationChallengeResponse>, AppError> {
    let email = normalize_email(&req.email);
    validate_email(&email)?;

    // Already has a key? Tell the client to authenticate instead.
    if db::email_has_credential(&state.db, &email).await? {
        return Err(AppError::Conflict("login"));
    }

    // Reuse the handle if a prior attempt created the user but never finished.
    let user_id = db::find_user_id_by_email(&state.db, &email)
        .await?
        .unwrap_or_else(Uuid::new_v4);

    session.remove::<RegState>(REG_STATE).await?;

    let (ccr, reg) = state
        .webauthn
        .start_securitykey_registration(user_id, &email, &email, None, None, None)?;

    session
        .insert(REG_STATE, RegState { user_id, email, reg })
        .await?;

    Ok(Json(ccr))
}

/// `POST /api/register/finish`
pub async fn register_finish(
    State(state): State<AppState>,
    session: Session,
    Json(cred): Json<RegisterPublicKeyCredential>,
) -> Result<Json<AuthenticatedResponse>, AppError> {
    let RegState { user_id, email, reg } = session
        .remove::<RegState>(REG_STATE)
        .await?
        .ok_or_else(|| AppError::BadRequest("no registration in progress".into()))?;

    let key = state.webauthn.finish_securitykey_registration(&cred, &reg)?;
    db::insert_user_and_credential(&state.db, user_id, &email, &key).await?;

    session
        .insert(AUTHED_USER, AuthedUser { user_id, email })
        .await?;
    Ok(Json(AuthenticatedResponse { authenticated: true }))
}

/// `POST /api/login/start`
pub async fn login_start(
    State(state): State<AppState>,
    session: Session,
    Json(req): Json<EmailRequest>,
) -> Result<Json<RequestChallengeResponse>, AppError> {
    let email = normalize_email(&req.email);
    validate_email(&email)?;

    let user_id = db::find_user_id_by_email(&state.db, &email)
        .await?
        .ok_or(AppError::NotFound("register"))?;

    let keys = db::load_securitykeys(&state.db, user_id).await?;
    if keys.is_empty() {
        return Err(AppError::NotFound("register"));
    }

    session.remove::<AuthState>(AUTH_STATE).await?;

    let (rcr, auth) = state.webauthn.start_securitykey_authentication(&keys)?;
    session
        .insert(AUTH_STATE, AuthState { user_id, auth })
        .await?;

    Ok(Json(rcr))
}

/// `POST /api/login/finish`
pub async fn login_finish(
    State(state): State<AppState>,
    session: Session,
    Json(cred): Json<PublicKeyCredential>,
) -> Result<Json<AuthenticatedResponse>, AppError> {
    let AuthState { user_id, auth } = session
        .remove::<AuthState>(AUTH_STATE)
        .await?
        .ok_or_else(|| AppError::BadRequest("no authentication in progress".into()))?;

    // webauthn-rs performs counter-regression / replay detection internally.
    let result = state.webauthn.finish_securitykey_authentication(&cred, &auth)?;

    // Bump the stored counter if the authenticator advanced it.
    if result.needs_update() {
        let mut keys = db::load_securitykeys(&state.db, user_id).await?;
        if let Some(key) = keys.iter_mut().find(|k| k.cred_id() == result.cred_id())
            && key.update_credential(&result).is_some()
        {
            db::update_credential(&state.db, key, result.counter()).await?;
        }
    }

    let email = db::email_for_user(&state.db, user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    session
        .insert(AUTHED_USER, AuthedUser { user_id, email })
        .await?;
    Ok(Json(AuthenticatedResponse { authenticated: true }))
}

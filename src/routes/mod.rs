pub mod export;
pub mod health;
pub mod signup;
pub mod webauthn;

use axum::Router;
use axum::routing::{get, post};
use time::Duration;
use tower_http::trace::TraceLayer;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

use crate::assets;
use crate::state::AppState;

/// Build the full application router with the session + tracing layers applied.
pub fn build_router(state: AppState, store: SqliteStore, session_secure: bool) -> Router {
    let session_layer = SessionManagerLayer::new(store)
        .with_secure(session_secure)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(30)));

    Router::new()
        .route("/", get(assets::index))
        .route("/healthz", get(health::healthz))
        .route("/api/register/start", post(webauthn::register_start))
        .route("/api/register/finish", post(webauthn::register_finish))
        .route("/api/login/start", post(webauthn::login_start))
        .route("/api/login/finish", post(webauthn::login_finish))
        .route("/api/signup", post(signup::submit))
        .route("/api/export", get(export::export))
        .fallback(assets::static_handler)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

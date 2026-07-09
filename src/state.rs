use std::sync::Arc;

use sqlx::SqlitePool;
use webauthn_rs::prelude::Webauthn;

/// Shared application state. Cheap to clone: pool is an `Arc` internally, the
/// other fields are `Arc`-wrapped.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub webauthn: Arc<Webauthn>,
    pub export_token: Arc<str>,
    pub admin_token: Arc<str>,
}

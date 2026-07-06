//! Server-side session state. All three values live in the `tower-sessions`
//! SQLite store; only an opaque session-id cookie ever reaches the browser.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::{SecurityKeyAuthentication, SecurityKeyRegistration};

/// In-flight WebAuthn registration ceremony state.
pub const REG_STATE: &str = "reg_state";
/// In-flight WebAuthn authentication ceremony state.
pub const AUTH_STATE: &str = "auth_state";
/// Marks the session as authenticated once a ceremony succeeds.
pub const AUTHED_USER: &str = "authed_user";

#[derive(Serialize, Deserialize)]
pub struct RegState {
    pub user_id: Uuid,
    pub email: String,
    pub reg: SecurityKeyRegistration,
}

#[derive(Serialize, Deserialize)]
pub struct AuthState {
    pub user_id: Uuid,
    pub auth: SecurityKeyAuthentication,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthedUser {
    pub user_id: Uuid,
    pub email: String,
}

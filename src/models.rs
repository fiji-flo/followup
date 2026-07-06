use serde::{Deserialize, Serialize};

/// `{ "email": "..." }` — used to begin both register and login ceremonies.
#[derive(Deserialize)]
pub struct EmailRequest {
    pub email: String,
}

/// Response returned when a ceremony completes and the session is now authenticated.
#[derive(Serialize)]
pub struct AuthenticatedResponse {
    pub authenticated: bool,
}

/// The signup form payload. `email` is intentionally NOT accepted here — it is
/// taken from the authenticated session so it stays bound to the security key.
#[derive(Deserialize)]
pub struct SignupRequest {
    pub full_name: String,
    pub company: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country: String,
    pub gdpr_consent: bool,
}

/// One row of the token-protected export.
#[derive(Serialize, sqlx::FromRow)]
pub struct SignupExport {
    pub id: i64,
    pub email: String,
    pub full_name: String,
    pub company: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country: String,
    pub gdpr_consent: bool,
    pub created_at: String,
}

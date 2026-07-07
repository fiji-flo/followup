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

/// The current signup for a user, returned by `GET /api/signup` to pre-fill the edit form.
#[derive(Serialize, sqlx::FromRow)]
pub struct SignupData {
    pub full_name: String,
    pub company: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country: String,
    pub gdpr_consent: bool,
}

/// One row of the token-protected export: every registered security key, with its
/// signup details if the person completed the form (`signed_up` distinguishes the two).
#[derive(Serialize, sqlx::FromRow)]
pub struct Registration {
    pub email: String,
    pub registered_at: String,
    pub signed_up: bool,
    pub full_name: Option<String>,
    pub company: Option<String>,
    pub street: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub gdpr_consent: Option<bool>,
    pub signed_up_at: Option<String>,
}

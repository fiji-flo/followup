//! SQLite access layer: pool setup, migrations, and typed queries.

use std::str::FromStr;

use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;
use webauthn_rs::prelude::Passkey;

use crate::models::{Registration, SignupData, SignupRequest};

/// Create the connection pool, ensuring the DB file's parent directory exists,
/// enabling WAL + foreign keys, then running embedded migrations.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    ensure_parent_dir(database_url)?;

    let opts = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

/// SQLite will not create intermediate directories for the DB file, so do it here.
fn ensure_parent_dir(database_url: &str) -> std::io::Result<()> {
    let path = database_url
        .trim_start_matches("sqlite://")
        .trim_start_matches("sqlite:");
    let path = path.split('?').next().unwrap_or(path);
    if path.is_empty() || path == ":memory:" {
        return Ok(());
    }
    if let Some(parent) = std::path::Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// True if the email already has at least one registered credential.
pub async fn email_has_credential(pool: &SqlitePool, email: &str) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credentials c \
         JOIN users u ON u.user_id = c.user_id \
         WHERE u.email = ?",
    )
    .bind(email)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

/// Look up the WebAuthn user handle for an email, if the user exists.
pub async fn find_user_id_by_email(
    pool: &SqlitePool,
    email: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar("SELECT user_id FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await
}

/// Fetch the email bound to a user handle.
pub async fn email_for_user(
    pool: &SqlitePool,
    user_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT email FROM users WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

/// Load and deserialize all passkeys registered to a user.
pub async fn load_passkeys(pool: &SqlitePool, user_id: Uuid) -> Result<Vec<Passkey>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT passkey FROM credentials WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|json| serde_json::from_str::<Passkey>(json).map_err(Into::into))
        .collect()
}

/// Persist a freshly registered user + credential in a single transaction.
pub async fn insert_user_and_credential(
    pool: &SqlitePool,
    user_id: Uuid,
    email: &str,
    passkey: &Passkey,
) -> Result<()> {
    let now = now_rfc3339();
    let passkey_json = serde_json::to_string(passkey)?;
    let cred_id = passkey.cred_id().as_ref();

    let mut tx = pool.begin().await?;
    sqlx::query("INSERT OR IGNORE INTO users (user_id, email, created_at) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(email)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO credentials (cred_id, user_id, passkey, counter, created_at) \
         VALUES (?, ?, ?, 0, ?)",
    )
    .bind(cred_id)
    .bind(user_id)
    .bind(&passkey_json)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Persist an updated passkey (counter bump) after a successful authentication.
pub async fn update_credential(pool: &SqlitePool, passkey: &Passkey, counter: u32) -> Result<()> {
    let json = serde_json::to_string(passkey)?;
    sqlx::query(
        "UPDATE credentials SET passkey = ?, counter = ?, last_used_at = ? WHERE cred_id = ?",
    )
    .bind(&json)
    .bind(counter as i64)
    .bind(now_rfc3339())
    .bind(passkey.cred_id().as_ref())
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert or update the signup for a user (one per attendee).
pub async fn upsert_signup(
    pool: &SqlitePool,
    user_id: Uuid,
    email: &str,
    req: &SignupRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO signups \
            (user_id, email, full_name, company, street, postal_code, city, country, gdpr_consent, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(user_id) DO UPDATE SET \
            email = excluded.email, full_name = excluded.full_name, company = excluded.company, \
            street = excluded.street, postal_code = excluded.postal_code, city = excluded.city, \
            country = excluded.country, gdpr_consent = excluded.gdpr_consent, created_at = excluded.created_at",
    )
    .bind(user_id)
    .bind(email)
    .bind(&req.full_name)
    .bind(&req.company)
    .bind(&req.street)
    .bind(&req.postal_code)
    .bind(&req.city)
    .bind(&req.country)
    .bind(req.gdpr_consent)
    .bind(now_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

/// The current signup for a user, if they have completed the form.
pub async fn get_signup(
    pool: &SqlitePool,
    user_id: Uuid,
) -> Result<Option<SignupData>, sqlx::Error> {
    sqlx::query_as::<_, SignupData>(
        "SELECT full_name, company, street, postal_code, city, country, gdpr_consent \
         FROM signups WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Whether phase 2 (login + the contact-info form) has been activated by the admin.
pub async fn phase2_active(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let active: i64 = sqlx::query_scalar("SELECT phase2_active FROM app_phase WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok(active != 0)
}

/// Flip the phase 2 switch. Idempotent.
pub async fn set_phase2_active(pool: &SqlitePool, active: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE app_phase SET phase2_active = ? WHERE id = 1")
        .bind(active)
        .execute(pool)
        .await?;
    Ok(())
}

/// Every registered security key, newest first, with its signup details if present —
/// for the token-protected export. A `NULL`-joined row means the key was registered but
/// the person has not completed the signup form yet.
pub async fn all_registrations(pool: &SqlitePool) -> Result<Vec<Registration>, sqlx::Error> {
    sqlx::query_as::<_, Registration>(
        "SELECT u.email AS email, \
                u.created_at AS registered_at, \
                CASE WHEN s.user_id IS NOT NULL THEN 1 ELSE 0 END AS signed_up, \
                s.full_name AS full_name, s.company AS company, s.street AS street, \
                s.postal_code AS postal_code, s.city AS city, s.country AS country, \
                s.gdpr_consent AS gdpr_consent, s.created_at AS signed_up_at \
         FROM users u \
         LEFT JOIN signups s ON s.user_id = u.user_id \
         ORDER BY u.created_at DESC",
    )
    .fetch_all(pool)
    .await
}

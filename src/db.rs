//! SQLite access layer: pool setup, migrations, and typed queries.

use std::str::FromStr;

use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;
use webauthn_rs::prelude::SecurityKey;

use crate::models::{SignupExport, SignupRequest};

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

/// Load and deserialize all security keys registered to a user.
pub async fn load_securitykeys(pool: &SqlitePool, user_id: Uuid) -> Result<Vec<SecurityKey>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT passkey FROM credentials WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|json| serde_json::from_str::<SecurityKey>(json).map_err(Into::into))
        .collect()
}

/// Persist a freshly registered user + credential in a single transaction.
pub async fn insert_user_and_credential(
    pool: &SqlitePool,
    user_id: Uuid,
    email: &str,
    key: &SecurityKey,
) -> Result<()> {
    let now = now_rfc3339();
    let passkey_json = serde_json::to_string(key)?;
    let cred_id = key.cred_id().as_ref();

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
pub async fn update_credential(pool: &SqlitePool, key: &SecurityKey, counter: u32) -> Result<()> {
    let json = serde_json::to_string(key)?;
    sqlx::query(
        "UPDATE credentials SET passkey = ?, counter = ?, last_used_at = ? WHERE cred_id = ?",
    )
    .bind(&json)
    .bind(counter as i64)
    .bind(now_rfc3339())
    .bind(key.cred_id().as_ref())
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

/// All signups, newest first — for the token-protected export.
pub async fn all_signups(pool: &SqlitePool) -> Result<Vec<SignupExport>, sqlx::Error> {
    sqlx::query_as::<_, SignupExport>(
        "SELECT id, email, full_name, company, street, postal_code, city, country, \
                gdpr_consent, created_at \
         FROM signups ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
}

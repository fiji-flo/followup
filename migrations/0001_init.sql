-- One row per attendee email. user_id is the WebAuthn user handle (UUID bytes).
CREATE TABLE IF NOT EXISTS users (
    user_id    BLOB PRIMARY KEY,
    email      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    created_at TEXT NOT NULL
);

-- WebAuthn credentials. The serialized Passkey (JSON) is the source of truth;
-- counter/last_used_at are convenience columns for auditing.
CREATE TABLE IF NOT EXISTS credentials (
    cred_id      BLOB PRIMARY KEY,
    user_id      BLOB NOT NULL,
    passkey      TEXT NOT NULL,
    counter      INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    last_used_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_credentials_user ON credentials(user_id);

-- The signup form payload. UNIQUE(user_id) => one signup per attendee;
-- re-submission updates the existing row via ON CONFLICT.
CREATE TABLE IF NOT EXISTS signups (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      BLOB NOT NULL UNIQUE,
    email        TEXT NOT NULL,
    full_name    TEXT NOT NULL,
    company      TEXT NOT NULL,
    street       TEXT NOT NULL,
    postal_code  TEXT NOT NULL,
    city         TEXT NOT NULL,
    country      TEXT NOT NULL,
    gdpr_consent INTEGER NOT NULL,
    created_at   TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_signups_user ON signups(user_id);

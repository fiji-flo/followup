# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Firefox Enterprise event signup service. At an event, FIDO2 security keys are handed out;
attendees visit the single page, verify with their key (WebAuthn), and submit a signup form
(name, company, shipping address, GDPR consent). Requiring the key proves attendance. Signups
are pulled via a token-protected JSON export.

Single self-contained Rust binary: **axum + SQLite (sqlx) + webauthn-rs**. The frontend
(`frontend/`, plain HTML/CSS/JS) and DB migrations (`migrations/`) are **compiled into the
binary** — via `rust-embed` and `sqlx::migrate!()` respectively — so there are no runtime file
dependencies beyond a writable path for the SQLite DB.

## Commands

Standard build/test on a normal machine is just `cargo build` / `cargo test` / `cargo clippy`.

**Under the Claude Code sandbox, three env vars are required** (cargo can't write `~/.cargo`,
`webauthn-rs` needs system OpenSSL which is not vendored):

```sh
CARGO_HOME="$TMPDIR/fu-cargo" \
OPENSSL_DIR=/opt/homebrew/opt/openssl@3 \
PKG_CONFIG_PATH=/opt/homebrew/opt/openssl@3/lib/pkgconfig \
cargo build   # or test / clippy
```

- Run a single test: `cargo test <name>` (e.g. `cargo test export_requires_token`). All tests
  live in `src/tests.rs`.
- **The server cannot be run live in the sandbox** — `TcpListener::bind` is blocked. Verify HTTP
  behavior with `cargo test`; the integration tests drive the router via `tower::ServiceExt::oneshot`
  (no socket). Startup up to the bind (config parse, WebAuthn build, DB + session migrations) does run.
- When backgrounding a piped cargo command, the exit code reflects the pipe, not cargo — read the
  output tail to confirm success.

Run locally (outside sandbox): `cp .env.example .env && cargo run`, open http://localhost:8080.
`localhost` is a WebAuthn secure context, so keys work over plain HTTP in dev; use Chrome DevTools →
WebAuthn → virtual authenticator to test without a physical key.

## Architecture

Request flow: `main.rs` loads `Config::from_env()`, builds the `Webauthn` instance and the SQLite
pool, then `routes::build_router` wires handlers + the session and tracing layers with `AppState`
(cheap-to-clone: pool is an internal `Arc`, other fields `Arc`-wrapped).

- `config.rs` — all config from env (no defaults in code; `.env` loaded via dotenvy). Validates that
  `RP_ID` is a registrable suffix of `RP_ORIGIN`'s host and that `EXPORT_TOKEN` is non-empty.
- `routes/webauthn.rs` — the four ceremony endpoints (`register/login` × `start/finish`).
- `routes/signup.rs` — `GET`/`POST /api/signup`, session-gated.
- `routes/export.rs` — `GET /api/export`, bearer-token-gated.
- `db.rs` — the entire SQLite access layer (pool setup, migrations, typed queries). No repository
  abstraction; handlers call these functions directly.
- `session.rs` — the three server-side session values and their string keys.
- `error.rs` — `AppError`, the single error type every handler returns.
- `assets.rs` — serves the embedded frontend.

### Auth model (the core design)

There are **no passwords and no separate signup step for identity** — the email is bound to a
security key via WebAuthn, and that binding *is* the identity.

- The frontend posts a single email. A **brand-new email registers** a key; a **known email
  authenticates**. The branch is client-driven: `register/start` returns `409 {action:"login"}`
  if the email already has a credential, and `login/start` returns `404 {action:"register"}` if it
  doesn't — `frontend/app.js`'s `verify()` tries register first and falls back to login on the 409.
- A successful ceremony writes `AUTHED_USER` into the session. Signup endpoints require it. **The
  email for a signup is always read from the session, never trusted from the request body** (see
  `SignupRequest`, which deliberately omits `email`).
- **All WebAuthn ceremony state and the authenticated-user marker live server-side** in the
  `tower-sessions` SQLite store; only an opaque session-id cookie reaches the browser.
- Export auth is separate: a bearer token compared in **constant time** (`subtle::ConstantTimeEq`),
  no session involved.

### Data model (`migrations/0001_init.sql`)

- `users` — one row per email; `user_id` is the WebAuthn user handle (UUID stored as BLOB).
- `credentials` — the serialized `Passkey` JSON is the source of truth; `counter`/`last_used_at`
  are convenience/audit columns. webauthn-rs does counter-regression/replay detection internally on
  finish; we persist the bumped counter only when `result.needs_update()`.
- `signups` — `UNIQUE(user_id)` enforces one signup per attendee; re-submission upserts via
  `ON CONFLICT`. `all_registrations` LEFT JOINs so the export lists **every registered key**, with a
  `signed_up` flag distinguishing keys that haven't completed the form (null signup columns) from
  those that have.

Timestamps are RFC3339 strings (`db::now_rfc3339`), not native SQLite dates.

## Conventions & gotchas

- **Dependency pins are load-bearing.** `sqlx` is pinned to `0.8` and `tower-sessions` to `0.14`
  because `tower-sessions-sqlx-store 0.15` depends on sqlx 0.8 — mixing versions puts two `sqlx` in
  the graph and the `SqlitePool` types stop matching. Don't bump these casually.
- **WebAuthn credential type is Passkey** (`start/finish_passkey_*`), which enforces user
  verification (PIN/biometric). Switching to the presence-only `SecurityKey` API touches
  `Cargo.toml`, `main.rs`, `session.rs`, `db.rs`, and `webauthn.rs` together — the stored-credential
  type and session-state type must match the active ceremony family or stored-credential JSON
  deserialization breaks.
- Internal errors (`Db`, `Session`, `Other`, `Webauthn`) are logged server-side and collapsed to a
  generic message so no implementation detail leaks to the client. `NotFound`/`Conflict` carry the
  `action` hint the frontend depends on.
- Frontend is framework-free and bilingual EN/DE (`frontend/i18n.js`). WebAuthn binary fields are
  base64url-encoded across the wire (helpers at the top of `app.js`).

# Firefox Enterprise — Event Signup Service

A small single-page web service. At an event we hand out FIDO2 security keys;
attendees visit this page, verify with their key (WebAuthn), and sign up to receive
Firefox Enterprise materials and swag at launch. Requiring the security key proves the
person actually attended.

- **Rust + axum + SQLite** (`sqlx`), single self-contained binary.
- Frontend (HTML/CSS/JS, no framework) and DB migrations are **compiled into the binary**.
- **WebAuthn** via `webauthn-rs` — passwordless; the email is bound to the security key.
  Uses the **Passkey** ceremony, which requires **user verification** (the authenticator
  must confirm the user via PIN or biometric), so keys without a PIN will prompt the user to
  set one during registration.
- Bilingual **EN/DE** with a language toggle.
- Collects name, company, and shipping address (with a GDPR consent checkbox).
- Signups are retrieved via a **token-protected JSON endpoint**.

## Configure

Copy `.env.example` to `.env` and adjust. Every value is read from the environment.

| Var | Meaning |
|-----|---------|
| `BIND_ADDR` | Address to listen on (plain HTTP; TLS is terminated by a reverse proxy). |
| `DATABASE_URL` | SQLite path, e.g. `sqlite://data/app.db?mode=rwc`. |
| `RP_ID` | WebAuthn RP ID — the bare host (must be a suffix of `RP_ORIGIN`'s host). |
| `RP_ORIGIN` | Full origin URL the browser sees, e.g. `https://enterprise.firefox.com`. |
| `RP_NAME` | Human-readable relying-party name. |
| `EXPORT_TOKEN` | Bearer token guarding `GET /api/export`. |
| `SESSION_SECURE` | `Secure` flag on the session cookie — `false` for local http, `true` behind TLS. |
| `RUST_LOG` | Log filter, e.g. `info`. |

## Run locally

`localhost` is a WebAuthn secure context, so security keys work over plain HTTP in dev.

```sh
cp .env.example .env      # defaults target http://localhost:8080
cargo run
```

Open http://localhost:8080. No physical key? Use Chrome DevTools → ⋮ → More tools →
**WebAuthn** → *Enable virtual authenticator environment* to test the full flow.

## Retrieve signups

```sh
curl -H "Authorization: Bearer $EXPORT_TOKEN" http://localhost:8080/api/export
```

Returns a JSON array of **every registered security key**, newest first. Each entry has
`email`, `registered_at`, and `signed_up` (bool); the signup fields (`full_name`, `company`,
address, `gdpr_consent`, `signed_up_at`) are populated once the person completes the form and
`null` otherwise. A missing/incorrect token returns `401`.

## HTTP API

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| GET | `/` | — | Landing page |
| POST | `/api/register/start` \| `/finish` | session | Register a new security key |
| POST | `/api/login/start` \| `/finish` | session | Authenticate a known key |
| GET | `/api/signup` | session | Current user's signup (pre-fills the edit form) |
| POST | `/api/signup` | session | Submit/update the signup form (upsert — one per attendee) |
| GET | `/api/export` | bearer token | Every registered key + signup details as JSON |
| GET | `/healthz` | — | Liveness probe |

A new email registers a key; a known email authenticates. The frontend switches
automatically based on the `action` hint returned by the start endpoints.

## Deploy

Build the release binary (`cargo build --release`) or the container image
(`docker build -t followup .`). Run it behind a reverse proxy (nginx/Caddy/…) that
terminates TLS and forwards to the app, and set `SESSION_SECURE=true` with the real
`RP_ID` / `RP_ORIGIN`. The binary is self-contained; it only needs a writable path for
the SQLite file.

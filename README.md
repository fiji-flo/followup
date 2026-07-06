# Firefox Enterprise — Event Signup Service

A small single-page web service. At an event we hand out FIDO2 security keys;
attendees visit this page, verify with their key (WebAuthn), and sign up to receive
Firefox Enterprise materials and swag at launch. Requiring the security key proves the
person actually attended.

- **Rust + axum + SQLite** (`sqlx`), single self-contained binary.
- Frontend (HTML/CSS/JS, no framework) and DB migrations are **compiled into the binary**.
- **WebAuthn** via `webauthn-rs` — the email is bound to the security key.
  Uses the **SecurityKey** ceremony with *user-presence-only* mode
  (`danger_set_user_presence_only_security_keys`), so a plain **touch** works even on keys
  with no PIN/biometric. This proves possession (they attended the event) without requiring
  user verification. If you'd rather require a PIN/biometric, switch to the `*_passkey_*`
  ceremonies, which enforce user verification.
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

Returns a JSON array of all signups. A missing/incorrect token returns `401`.

## HTTP API

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| GET | `/` | — | Landing page |
| POST | `/api/register/start` \| `/finish` | session | Register a new security key |
| POST | `/api/login/start` \| `/finish` | session | Authenticate a known key |
| POST | `/api/signup` | session | Submit the signup form |
| GET | `/api/export` | bearer token | All signups as JSON |
| GET | `/healthz` | — | Liveness probe |

A new email registers a key; a known email authenticates. The frontend switches
automatically based on the `action` hint returned by the start endpoints.

## Deploy

Build the release binary (`cargo build --release`) or the container image
(`docker build -t followup .`). Run it behind a reverse proxy (nginx/Caddy/…) that
terminates TLS and forwards to the app, and set `SESSION_SECURE=true` with the real
`RP_ID` / `RP_ORIGIN`. The binary is self-contained; it only needs a writable path for
the SQLite file.

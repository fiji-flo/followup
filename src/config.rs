use std::net::SocketAddr;

use anyhow::{Context, Result, ensure};
use webauthn_rs::prelude::Url;

/// Runtime configuration, loaded once from the environment at startup.
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub rp_id: String,
    pub rp_origin: Url,
    pub rp_name: String,
    pub export_token: String,
    pub admin_token: String,
    pub session_secure: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env("BIND_ADDR")?
            .parse()
            .context("BIND_ADDR must be a valid socket address, e.g. 127.0.0.1:8080")?;
        let database_url = env("DATABASE_URL")?;
        let rp_id = env("RP_ID")?;
        let rp_origin = Url::parse(&env("RP_ORIGIN")?)
            .context("RP_ORIGIN must be a valid URL, e.g. https://enterprise.firefox.com")?;
        let rp_name = env("RP_NAME")?;
        let export_token = env("EXPORT_TOKEN")?;
        let admin_token = env("ADMIN_TOKEN")?;
        let session_secure = env("SESSION_SECURE")?
            .parse()
            .context("SESSION_SECURE must be `true` or `false`")?;

        ensure!(!export_token.is_empty(), "EXPORT_TOKEN must not be empty");
        ensure!(!admin_token.is_empty(), "ADMIN_TOKEN must not be empty");
        ensure!(
            rp_origin
                .host_str()
                .is_some_and(|h| h == rp_id || h.ends_with(&format!(".{rp_id}"))),
            "RP_ID ({rp_id}) must be a registrable suffix of RP_ORIGIN's host ({:?})",
            rp_origin.host_str()
        );

        Ok(Self {
            bind_addr,
            database_url,
            rp_id,
            rp_origin,
            rp_name,
            export_token,
            admin_token,
            session_secure,
        })
    }
}

fn env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("missing required environment variable {key}"))
}

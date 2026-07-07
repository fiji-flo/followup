mod assets;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod session;
mod state;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tower_sessions_sqlx_store::SqliteStore;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};
use webauthn_rs::prelude::WebauthnBuilder;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer())
        .init();

    let config = Config::from_env()?;

    let webauthn = WebauthnBuilder::new(&config.rp_id, &config.rp_origin)
        .context("invalid WebAuthn RP configuration")?
        .rp_name(&config.rp_name)
        .build()
        .context("failed to build WebAuthn instance")?;

    let pool = db::init_pool(&config.database_url)
        .await
        .context("failed to initialize database")?;

    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .context("failed to migrate session store")?;

    let state = AppState {
        db: pool,
        webauthn: Arc::new(webauthn),
        export_token: Arc::from(config.export_token.as_str()),
    };

    let app = routes::build_router(state, session_store, config.session_secure);

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    tracing::info!("listening on http://{}", config.bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

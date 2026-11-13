use std::net::SocketAddr;

use anyhow::Context;
use malmoon_auth::{Auth, AuthConfig, LogMailer};
use malmoon_gateway::{Config, router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("malmoon_gateway=info,malmoon_auth=info,tower_http=info")
        }))
        .init();

    let addr: SocketAddr = std::env::var("MALMOON_GATEWAY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_owned())
        .parse()
        .context("MALMOON_GATEWAY_ADDR must be an address such as 127.0.0.1:8787")?;
    let config = Config {
        demo_captions: std::env::var("MALMOON_DEMO_CAPTIONS").is_ok_and(|value| value == "1"),
    };
    if config.demo_captions {
        tracing::warn!("demo captions are on: every session receives a scripted feed");
    }

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must point at Postgres; `mise run db:up` starts a local one")?;
    let auth_config = AuthConfig::from_env()?;
    // No mail provider is wired up yet, so sign-in codes go to the log. Anyone who can read the
    // log can sign in as anyone: this must be replaced before the gateway is deployed.
    tracing::warn!("sign-in codes are written to the log instead of being emailed");
    let auth = Auth::connect(&database_url, auth_config, LogMailer).await?;
    tracing::info!(providers = ?auth.providers(), public_url = %auth.config().public_url, "accounts ready");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("could not listen on {addr}"))?;
    tracing::info!(%addr, "gateway listening");
    axum::serve(listener, router(config, auth))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

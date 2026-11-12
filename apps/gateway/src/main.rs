use std::net::SocketAddr;

use anyhow::Context;
use malmoon_gateway::{Config, router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("malmoon_gateway=info,tower_http=info")),
        )
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

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("could not listen on {addr}"))?;
    tracing::info!(%addr, "gateway listening");
    axum::serve(listener, router(config))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

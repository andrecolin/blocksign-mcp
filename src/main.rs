//! blocksign MCP server — main binary entry.

use std::net::SocketAddr;

use anyhow::Context;
use blocksign_mcp_server::{config::Config, router};
use tokio::net::TcpListener;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env();

    fmt()
        .with_env_filter(EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(false)
        .init();

    let bind: SocketAddr = config
        .bind_addr
        .parse()
        .context("invalid MCP_BIND address")?;

    let state = router::AppState::with_public_url(&config.api_base_url, &config.public_url);
    let app = router::build(state);

    tracing::info!(
        %bind,
        api_base = %config.api_base_url,
        public_url = %config.public_url,
        "blocksign-mcp-server listening (POST /mcp for JSON-RPC)"
    );

    let listener = TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

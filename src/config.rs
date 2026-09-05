//! Runtime configuration for the MCP server.

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub api_base_url: String,
    /// Public URL agents should use to reach this server — advertised in the
    /// discovery document. Prod: `https://mcp.blocksign.red/mcp`.
    pub public_url: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("MCP_BIND").unwrap_or_else(|_| "0.0.0.0:8200".into()),
            api_base_url: env::var("BLOCKSIGN_API_URL")
                .unwrap_or_else(|_| "http://localhost:8085".into()),
            public_url: env::var("MCP_PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:8200/mcp".into()),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        }
    }
}

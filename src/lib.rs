//! blocksign MCP server library.
//!
//! Implements the Model Context Protocol (2025-06-18 Streamable HTTP, with
//! version negotiation down to 2024-11-05) as a thin
//! translation layer between MCP clients (Claude Desktop, etc.) and the
//! blocksign REST API.
//!
//! Architecture:
//! - HTTP transport: `POST /mcp` accepts JSON-RPC 2.0 requests
//! - Auth: `Authorization: Bearer bsk_agent_*` forwarded as-is to the API
//! - No database connection — all state lives in the blocksign API

pub mod api_client;
pub mod config;
pub mod prompts;
pub mod protocol;
pub mod resources;
pub mod router;
pub mod tools;

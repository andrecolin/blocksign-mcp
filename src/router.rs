//! Streamable HTTP transport for the MCP server (spec 2025-06-18).
//!
//! - `POST /mcp` — JSON-RPC 2.0 endpoint. We always answer with a single
//!   `application/json` body (spec-compliant; SSE streaming is optional and
//!   our tools are strict request/response). `Mcp-Session-Id` is issued on
//!   `initialize` and echoed thereafter; `MCP-Protocol-Version` is echoed.
//! - `GET  /mcp` — 405 (we have no server-initiated messages)
//! - `GET  /health` — liveness probe
//! - `GET  /.well-known/mcp.json` — server discovery metadata
//! - `GET  /.well-known/oauth-protected-resource` — RFC 9728 metadata
//!   (kept shape-compatible with the API's copy in
//!   services/api/src/routes/discovery.rs)

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::WWW_AUTHENTICATE, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::api_client::ApiClient;
use crate::protocol::{self, JsonRpcRequest, JsonRpcResponse, AUTH_REQUIRED, PARSE_ERROR};

const SESSION_HEADER: &str = "mcp-session-id";
const VERSION_HEADER: &str = "mcp-protocol-version";

#[derive(Clone)]
pub struct AppState {
    pub api: Arc<ApiClient>,
    /// Public URL of this server's /mcp endpoint, advertised in discovery.
    pub public_url: Arc<str>,
}

impl AppState {
    pub fn new(api_base_url: &str) -> Self {
        Self::with_public_url(api_base_url, "http://localhost:8200/mcp")
    }

    pub fn with_public_url(api_base_url: &str, public_url: &str) -> Self {
        Self {
            api: Arc::new(ApiClient::new(api_base_url)),
            public_url: Arc::from(public_url),
        }
    }
}

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/mcp", post(mcp_handler).get(mcp_get))
        .route("/.well-known/mcp.json", get(discovery))
        .route(
            "/.well-known/oauth-protected-resource",
            get(oauth_protected_resource),
        )
        .with_state(state)
}

/// GET /mcp — the Streamable HTTP spec lets clients open a GET for
/// server-initiated SSE; we have none, and 405 is the spec-sanctioned reply.
async fn mcp_get() -> impl IntoResponse {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [("allow", "POST")],
        "this MCP server has no server-initiated messages; POST JSON-RPC to /mcp",
    )
}

/// RFC 9728 OAuth protected-resource metadata. blocksign uses long-lived
/// bearer API keys (bsk_agent_*), not OAuth token exchange — this document
/// tells OAuth-aware MCP clients where credentials come from.
async fn oauth_protected_resource(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "resource": &*state.public_url,
        "authorization_servers": [],
        "bearer_methods_supported": ["header"],
        "scopes_supported": [
            "agreements.create", "agreements.read", "agreements.void",
            "documents.upload", "templates.read", "billing.read",
            "billing.purchase", "billing.auto_purchase"
        ],
        "resource_name": "blocksign.red MCP server",
        "resource_documentation": "https://blocksign.red/llms-full.txt",
        "credential_issuance": {
            "type": "api_key",
            "key_prefix": "bsk_agent_",
            "issue_url": "https://blocksign.red/settings/api-keys",
            "issue_api": "POST /v1/api-keys (JWT auth)",
            "description": "Issue an agent-scoped key, then send Authorization: Bearer <key>."
        }
    }))
}

async fn health() -> &'static str {
    "ok"
}

async fn discovery(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "name": "blocksign-mcp-server",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol_version": protocol::PROTOCOL_VERSION,
        "transport": "streamable-http",
        "endpoint": &*state.public_url,
        "authentication": {
            "type": "bearer",
            "description": "Pass your bsk_agent_* API key as Authorization: Bearer <key>"
        }
    }))
}

async fn mcp_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Json<Value>,
) -> Response {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Echo the negotiated protocol version header (2025-06-18 transport);
    // absent header means a pre-2025 client — answer with our latest.
    let protocol_version = headers
        .get(VERSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|v| protocol::SUPPORTED_VERSIONS.contains(v))
        .unwrap_or(protocol::PROTOCOL_VERSION)
        .to_string();

    let req: JsonRpcRequest = match serde_json::from_value(body.0.clone()) {
        Ok(r) => r,
        Err(e) => {
            let resp = JsonRpcResponse::err(
                body.0.get("id").cloned(),
                PARSE_ERROR,
                format!("invalid JSON-RPC: {e}"),
            );
            return with_transport_headers(
                (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response(),
                &protocol_version,
                None,
            );
        }
    };

    // Session tracking: issue a fresh id on `initialize`, echo the client's
    // id otherwise. We hold no per-session state (the API does), so this is
    // purely the transport-level contract.
    let session_id = if req.method == "initialize" {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        headers
            .get(SESSION_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };

    let is_notification = req.id.is_none();
    let resp = protocol::handle_request(req, auth.as_deref(), &state.api).await;

    if is_notification {
        return with_transport_headers(
            StatusCode::NO_CONTENT.into_response(),
            &protocol_version,
            session_id.as_deref(),
        );
    }

    // Per the MCP authorization spec, a missing credential is an HTTP 401
    // carrying WWW-Authenticate that points at the RFC 9728 metadata. The
    // JSON-RPC error body is preserved for older clients.
    let status = match &resp.error {
        Some(e) if e.code == AUTH_REQUIRED => StatusCode::UNAUTHORIZED,
        _ => StatusCode::OK,
    };

    let mut response =
        (status, Json(serde_json::to_value(resp).unwrap())).into_response();
    if status == StatusCode::UNAUTHORIZED {
        let metadata_url = well_known_sibling(&state.public_url, "oauth-protected-resource");
        if let Ok(v) =
            format!(r#"Bearer resource_metadata="{metadata_url}""#).parse()
        {
            response.headers_mut().insert(WWW_AUTHENTICATE, v);
        }
    }
    with_transport_headers(response, &protocol_version, session_id.as_deref())
}

/// Attach `MCP-Protocol-Version` and (when known) `Mcp-Session-Id` headers.
fn with_transport_headers(
    mut response: Response,
    protocol_version: &str,
    session_id: Option<&str>,
) -> Response {
    if let Ok(v) = protocol_version.parse() {
        response.headers_mut().insert(VERSION_HEADER, v);
    }
    if let Some(sid) = session_id {
        if let Ok(v) = sid.parse() {
            response.headers_mut().insert(SESSION_HEADER, v);
        }
    }
    response
}

/// Build `<scheme>://<host>/.well-known/<suffix>` from any URL on that host.
fn well_known_sibling(url: &str, suffix: &str) -> String {
    let base = url.split('/').take(3).collect::<Vec<_>>().join("/");
    format!("{base}/.well-known/{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_ok() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let resp = app
            .oneshot(
                http::Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn discovery_endpoint_returns_metadata() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let resp = app
            .oneshot(
                http::Request::builder()
                    .uri("/.well-known/mcp.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["name"], "blocksign-mcp-server");
        assert_eq!(json["protocol_version"], protocol::PROTOCOL_VERSION);
        assert_eq!(json["endpoint"], "http://localhost:8200/mcp");
        assert_eq!(json["transport"], "streamable-http");
    }

    #[tokio::test]
    async fn initialize_via_http_returns_capabilities() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let req_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["result"]["protocolVersion"], protocol::PROTOCOL_VERSION);
    }

    #[tokio::test]
    async fn tools_list_via_http_returns_catalog() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let req_body = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let tools = json["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 7);
    }

    #[tokio::test]
    async fn invalid_jsonrpc_returns_error_with_id() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let req_body = json!({"jsonrpc": "2.0", "id": 99});
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], PARSE_ERROR);
        assert_eq!(json["id"], 99);
    }

    #[tokio::test]
    async fn initialize_issues_session_id_and_version_header() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let req_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        });
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get("mcp-session-id").is_some());
        assert_eq!(
            resp.headers().get("mcp-protocol-version").unwrap(),
            protocol::PROTOCOL_VERSION
        );
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        // Body-level negotiation echoes the client's older revision.
        assert_eq!(json["result"]["protocolVersion"], "2024-11-05");
    }

    #[tokio::test]
    async fn session_id_is_echoed_on_subsequent_requests() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let req_body = json!({"jsonrpc": "2.0", "id": 2, "method": "ping", "params": {}});
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header("mcp-session-id", "test-session-123")
                    .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get("mcp-session-id").unwrap(),
            "test-session-123"
        );
    }

    #[tokio::test]
    async fn missing_auth_returns_401_with_www_authenticate() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let req_body = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "list_templates"}
        });
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let www = resp
            .headers()
            .get("www-authenticate")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(www.contains("resource_metadata="));
        assert!(www.contains("/.well-known/oauth-protected-resource"));
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], AUTH_REQUIRED);
    }

    #[tokio::test]
    async fn get_mcp_returns_405() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let resp = app
            .oneshot(
                http::Request::builder()
                    .uri("/mcp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn oauth_protected_resource_describes_bearer_keys() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let resp = app
            .oneshot(
                http::Request::builder()
                    .uri("/.well-known/oauth-protected-resource")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["credential_issuance"]["key_prefix"], "bsk_agent_");
        assert_eq!(json["bearer_methods_supported"][0], "header");
    }

    #[tokio::test]
    async fn notification_returns_204() {
        let state = AppState::new("http://nowhere");
        let app = build(state);
        let req_body = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}

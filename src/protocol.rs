//! MCP protocol JSON-RPC 2.0 message types and dispatcher.
//!
//! Speaks 2025-06-18 (Streamable HTTP era) by default and negotiates down to
//! older revisions for backward compatibility — `initialize` echoes the
//! client's requested version when we support it.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api_client::ApiClient;
use crate::{prompts, resources, tools};

/// Latest MCP protocol version we speak (and the default when a client asks
/// for something we don't know).
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// All revisions we accept during `initialize` version negotiation.
pub const SUPPORTED_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 response envelope.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

// JSON-RPC error codes (per MCP/JSON-RPC 2.0 spec).
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;
/// Server-defined: request needs a bearer key. The HTTP transport maps this
/// to 401 + `WWW-Authenticate` per the MCP authorization spec (RFC 9728).
pub const AUTH_REQUIRED: i32 = -32001;

/// Dispatch a parsed MCP request to the right handler.
pub async fn handle_request(
    req: JsonRpcRequest,
    auth: Option<&str>,
    api: &ApiClient,
) -> JsonRpcResponse {
    let id = req.id.clone();

    if req.jsonrpc != "2.0" {
        return JsonRpcResponse::err(id, INVALID_REQUEST, "jsonrpc must be \"2.0\"");
    }

    match req.method.as_str() {
        "initialize" => handle_initialize(id, req.params),
        "notifications/initialized" => JsonRpcResponse::ok(id, json!({})),
        "ping" => JsonRpcResponse::ok(id, json!({})),
        "tools/list" => JsonRpcResponse::ok(id, tools::list_tools()),
        "tools/call" => {
            let auth = match auth {
                Some(a) => a,
                None => {
                    return JsonRpcResponse::err(
                        id,
                        AUTH_REQUIRED,
                        "missing Authorization header — pass bsk_agent_* key",
                    );
                }
            };
            match tools::call_tool(req.params, auth, api).await {
                Ok(v) => JsonRpcResponse::ok(id, v),
                Err(e) => JsonRpcResponse::err(id, INTERNAL_ERROR, e),
            }
        }
        "resources/list" => JsonRpcResponse::ok(id, resources::list_resources()),
        "resources/read" => {
            let auth = match auth {
                Some(a) => a,
                None => {
                    return JsonRpcResponse::err(
                        id,
                        AUTH_REQUIRED,
                        "missing Authorization header — pass bsk_agent_* key",
                    );
                }
            };
            match resources::read_resource(req.params, auth, api).await {
                Ok(v) => JsonRpcResponse::ok(id, v),
                Err(e) => JsonRpcResponse::err(id, INTERNAL_ERROR, e),
            }
        }
        "prompts/list" => JsonRpcResponse::ok(id, prompts::list_prompts()),
        "prompts/get" => match prompts::get_prompt(req.params) {
            Ok(v) => JsonRpcResponse::ok(id, v),
            Err(e) => JsonRpcResponse::err(id, INVALID_PARAMS, e),
        },
        other => JsonRpcResponse::err(id, METHOD_NOT_FOUND, format!("unknown method: {other}")),
    }
}

fn handle_initialize(id: Option<Value>, params: Value) -> JsonRpcResponse {
    // Version negotiation: echo the client's requested revision when we
    // support it; otherwise answer with the latest we speak (per spec the
    // client then decides whether to proceed or disconnect).
    let negotiated = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .filter(|v| SUPPORTED_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSION);

    let result = json!({
        "protocolVersion": negotiated,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "listChanged": false, "subscribe": false },
            "prompts": { "listChanged": false }
        },
        "serverInfo": {
            "name": "blocksign-mcp-server",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Use these tools to create blockchain-anchored e-signature agreements. Pass your bsk_agent_* API key as Authorization: Bearer in every call. Start with `list_templates` to discover built-in contract types, or `create_agreement` for a one-call signing flow."
    });
    JsonRpcResponse::ok(id, result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_returns_empty_result() {
        let api = ApiClient::new("http://nowhere");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "ping".into(),
            params: json!({}),
        };
        let resp = handle_request(req, None, &api).await;
        assert!(resp.error.is_none());
        assert_eq!(resp.result, Some(json!({})));
    }

    #[tokio::test]
    async fn initialize_returns_capabilities() {
        let api = ApiClient::new("http://nowhere");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: json!({}),
        };
        let resp = handle_request(req, None, &api).await;
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert!(result["capabilities"]["tools"].is_object());
        assert!(result["capabilities"]["resources"].is_object());
        assert!(result["capabilities"]["prompts"].is_object());
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let api = ApiClient::new("http://nowhere");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "unknown_method".into(),
            params: json!({}),
        };
        let resp = handle_request(req, None, &api).await;
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn tools_call_without_auth_returns_invalid_request() {
        let api = ApiClient::new("http://nowhere");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "tools/call".into(),
            params: json!({"name": "list_templates"}),
        };
        let resp = handle_request(req, None, &api).await;
        let err = resp.error.unwrap();
        assert_eq!(err.code, AUTH_REQUIRED);
        assert!(err.message.contains("Authorization"));
    }

    #[tokio::test]
    async fn initialize_echoes_supported_older_version() {
        let api = ApiClient::new("http://nowhere");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: json!({"protocolVersion": "2024-11-05"}),
        };
        let resp = handle_request(req, None, &api).await;
        assert_eq!(resp.result.unwrap()["protocolVersion"], "2024-11-05");
    }

    #[tokio::test]
    async fn initialize_falls_back_to_latest_on_unknown_version() {
        let api = ApiClient::new("http://nowhere");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: json!({"protocolVersion": "1999-01-01"}),
        };
        let resp = handle_request(req, None, &api).await;
        assert_eq!(resp.result.unwrap()["protocolVersion"], PROTOCOL_VERSION);
    }

    #[tokio::test]
    async fn wrong_jsonrpc_version_rejected() {
        let api = ApiClient::new("http://nowhere");
        let req = JsonRpcRequest {
            jsonrpc: "1.0".into(),
            id: Some(json!(1)),
            method: "ping".into(),
            params: json!({}),
        };
        let resp = handle_request(req, None, &api).await;
        assert_eq!(resp.error.unwrap().code, INVALID_REQUEST);
    }
}

//! MCP tools — each one wraps a blocksign REST API endpoint.

use serde_json::{json, Value};

use crate::api_client::{ApiClient, ApiError};

/// Static catalog returned by `tools/list`.
pub fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "create_agreement",
                "description": "Create and optionally send an agreement in a single call. Supports either a built-in template (with variables) or an uploaded PDF (document_base64). Signers can be individuals OR companies — set party_type='company' with company_name + signer_title to sign on behalf of a legal entity (corp-to-corp, corp-to-individual). Returns agreement_id, signing URLs, and verification URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "template": { "type": "string", "description": "Template slug (e.g. 'nda'). Use list_templates to discover." },
                        "variables": { "type": "object", "description": "Variables for the template (must match its schema)." },
                        "document_base64": { "type": "string", "description": "Base64-encoded PDF (alternative to template)." },
                        "document_title": { "type": "string" },
                        "signers": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string", "description": "The individual person who signs." },
                                    "email": { "type": "string", "format": "email" },
                                    "routing_order": { "type": "integer" },
                                    "party_type": { "type": "string", "enum": ["individual", "company"], "default": "individual", "description": "'company' = signs on behalf of a legal entity; requires company_name + signer_title." },
                                    "company_name": { "type": "string", "description": "Legal entity name (required when party_type='company'). E.g. 'Acme Construction, LLC'." },
                                    "signer_title": { "type": "string", "description": "Signatory's title/capacity (required when party_type='company'). E.g. 'CEO', 'Project Manager'." },
                                    "verification_level": { "type": "string", "enum": ["email", "email_sms", "id", "kyc"], "description": "Identity assurance. email=free; email_sms=Pro; id/kyc=Business." },
                                    "phone": { "type": "string", "description": "E.164 phone, required for email_sms." },
                                    "metadata": { "type": "object", "description": "Free-form per-signer key/values surfaced on the certificate, e.g. {\"license_number\":\"ABC123\",\"role\":\"subcontractor\"}." }
                                },
                                "required": ["name", "email"]
                            }
                        },
                        "options": {
                            "type": "object",
                            "properties": {
                                "auto_send": { "type": "boolean", "default": true },
                                "routing_mode": { "type": "string", "enum": ["sequential", "parallel"] },
                                "expires_in_days": { "type": "integer" },
                                "message": { "type": "string" },
                                "default_verification_level": { "type": "string", "enum": ["email", "email_sms", "id", "kyc"], "description": "Default identity assurance for signers that don't set their own." }
                            }
                        }
                    },
                    "required": ["signers"]
                }
            },
            {
                "name": "get_agreement_status",
                "description": "Fetch unified status for an agreement: envelope state, per-signer status, verification URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agreement_id": { "type": "string", "format": "uuid" }
                    },
                    "required": ["agreement_id"]
                }
            },
            {
                "name": "list_agreements",
                "description": "List recent agreements for the calling org.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 }
                    }
                }
            },
            {
                "name": "list_templates",
                "description": "List contract templates available to the calling org (org-owned + platform public).",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "verify_agreement",
                "description": "Public verification of an agreement's audit chain and on-chain anchor.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agreement_id": { "type": "string", "format": "uuid" }
                    },
                    "required": ["agreement_id"]
                }
            },
            {
                "name": "void_agreement",
                "description": "Void an envelope before all parties sign.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "envelope_id": { "type": "string", "format": "uuid" },
                        "reason": { "type": "string" }
                    },
                    "required": ["envelope_id"]
                }
            },
            {
                "name": "preview_template",
                "description": "Render a template with variables (no PDF, no envelope) — useful to validate inputs before create_agreement.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "slug": { "type": "string" },
                        "variables": { "type": "object" }
                    },
                    "required": ["slug", "variables"]
                }
            },
            {
                "name": "upload_document",
                "description": "Upload a document for use in agreements. Accepts plain text or Markdown — blocksign converts it to PDF. Optionally runs AI field detection and returns suggested signature positions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text_content": { "type": "string", "description": "Plain text or Markdown contract body." },
                        "format": { "type": "string", "enum": ["plain", "markdown"], "default": "plain" },
                        "title": { "type": "string", "description": "Document title." },
                        "detect_fields": { "type": "boolean", "default": false, "description": "Run AI field detection and include results in response." },
                        "signers_count": { "type": "integer", "minimum": 1, "default": 1 }
                    },
                    "required": ["text_content", "title"]
                }
            },
            {
                "name": "detect_fields",
                "description": "Run AI field detection on a previously uploaded text document. Returns suggested signature/initials/date field positions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": "string", "format": "uuid" },
                        "signers_count": { "type": "integer", "minimum": 1, "default": 1 }
                    },
                    "required": ["document_id"]
                }
            },
            {
                "name": "check_billing_status",
                "description": "Check current billing plan, credit balance, usage, and per-document pricing for the calling org.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "purchase_credits",
                "description": "Purchase a credit pack (10, 50, or 200 credits). If payment method on file, charges immediately. Otherwise returns a checkout URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "credits": { "type": "integer", "enum": [10, 50, 200], "description": "Number of credits to purchase." }
                    },
                    "required": ["credits"]
                }
            }
        ]
    })
}

/// Dispatch a `tools/call` request.
pub async fn call_tool(params: Value, auth: &str, api: &ApiClient) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing tool name".to_string())?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match name {
        "create_agreement" => call_api(api, "POST", "/v1/agent/agreements", auth, Some(&args)).await,
        "get_agreement_status" => {
            let id = require_str(&args, "agreement_id")?;
            call_api(api, "GET", &format!("/v1/agent/agreements/{id}"), auth, None).await
        }
        "list_agreements" => {
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(20);
            call_api(api, "GET", &format!("/v1/envelopes?limit={limit}"), auth, None).await
        }
        "list_templates" => call_api(api, "GET", "/v1/templates", auth, None).await,
        "verify_agreement" => {
            let id = require_str(&args, "agreement_id")?;
            call_api(api, "GET", &format!("/v1/verify/{id}"), auth, None).await
        }
        "void_agreement" => {
            let id = require_str(&args, "envelope_id")?;
            let body = json!({ "reason": args.get("reason").cloned().unwrap_or(Value::Null) });
            call_api(api, "POST", &format!("/v1/envelopes/{id}/void"), auth, Some(&body)).await
        }
        "preview_template" => {
            let slug = require_str(&args, "slug")?;
            let body = json!({ "variables": args.get("variables").cloned().unwrap_or(json!({})) });
            call_api(api, "POST", &format!("/v1/templates/{slug}/preview"), auth, Some(&body)).await
        }
        "upload_document" => {
            let text_content = require_str(&args, "text_content")?;
            let title = require_str(&args, "title")?;
            let format = args.get("format").and_then(Value::as_str).unwrap_or("plain");
            let detect = args.get("detect_fields").and_then(Value::as_bool).unwrap_or(false);
            let signers_count = args.get("signers_count").and_then(Value::as_u64).unwrap_or(1);

            let content_type = if format == "markdown" {
                "text/markdown"
            } else {
                "text/plain"
            };
            let file_name = if format == "markdown" { "document.md" } else { "document.txt" };

            let detect_str = if detect { "true".to_owned() } else { String::new() };
            let signers_str = signers_count.to_string();
            let mut text_fields: Vec<(&str, &str)> = vec![("title", title)];
            if detect {
                text_fields.push(("detect_fields", &detect_str));
                text_fields.push(("signers_count", &signers_str));
            }

            api.post_multipart(
                "/v1/documents",
                auth,
                &text_fields,
                text_content.as_bytes().to_vec(),
                file_name,
                content_type,
            )
            .await
            .map_err(|e| e.to_string())
        }
        "detect_fields" => {
            let id = require_str(&args, "document_id")?;
            let signers_count = args.get("signers_count").and_then(Value::as_u64).unwrap_or(1);
            let body = json!({ "document_id": id, "signers_count": signers_count });
            call_api(api, "POST", "/v1/agent/fields/detect", auth, Some(&body)).await
        }
        "check_billing_status" => {
            call_api(api, "GET", "/v1/billing/status", auth, None).await
        }
        "purchase_credits" => {
            let credits = args.get("credits").and_then(Value::as_i64).unwrap_or(10);
            let body = json!({ "credits": credits });
            call_api(api, "POST", "/v1/billing/credits/purchase", auth, Some(&body)).await
        }
        other => return Err(format!("unknown tool: {other}")),
    };

    match result {
        Ok(value) => Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
            }],
            "isError": false
        })),
        Err(e) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Error: {e}") }],
            "isError": true
        })),
    }
}

async fn call_api(
    api: &ApiClient,
    method: &str,
    path: &str,
    auth: &str,
    body: Option<&Value>,
) -> Result<Value, String> {
    let res: Result<Value, ApiError> = match (method, body) {
        ("GET", _) => api.get(path, auth).await,
        ("POST", Some(b)) => api.post(path, auth, b).await,
        ("POST", None) => api.post_empty(path, auth).await,
        _ => return Err(format!("unsupported method: {method}")),
    };
    res.map_err(|e| e.to_string())
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tools_returns_all_expected() {
        let v = list_tools();
        let names: Vec<&str> = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&"create_agreement"));
        assert!(names.contains(&"list_templates"));
        assert!(names.contains(&"verify_agreement"));
        assert!(names.contains(&"void_agreement"));
        assert!(names.contains(&"preview_template"));
        assert_eq!(names.len(), 11);
    }

    #[test]
    fn create_agreement_advertises_company_party_fields() {
        let v = list_tools();
        let create = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "create_agreement")
            .expect("create_agreement tool");
        let signer_props =
            &create["inputSchema"]["properties"]["signers"]["items"]["properties"];
        for key in ["party_type", "company_name", "signer_title", "verification_level", "metadata"] {
            assert!(
                signer_props.get(key).is_some(),
                "create_agreement signer schema must advertise '{key}' for corp-to-corp"
            );
        }
        // party_type enumerates individual + company.
        let pt = &signer_props["party_type"]["enum"];
        assert!(pt.as_array().unwrap().iter().any(|x| x == "company"));
    }

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let api = ApiClient::new("http://nowhere");
        let result = call_tool(json!({"name": "doesnt_exist"}), "Bearer x", &api).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn missing_tool_name_returns_error() {
        let api = ApiClient::new("http://nowhere");
        let result = call_tool(json!({}), "Bearer x", &api).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("name"));
    }

    #[test]
    fn require_str_validates_argument() {
        let args = json!({"agreement_id": "abc"});
        assert_eq!(require_str(&args, "agreement_id").unwrap(), "abc");
        assert!(require_str(&args, "missing").is_err());
    }

    #[test]
    fn upload_document_in_tool_list() {
        let v = list_tools();
        let names: Vec<&str> = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(
            names.contains(&"upload_document"),
            "upload_document should be in the tool list"
        );
    }

    #[test]
    fn detect_fields_in_tool_list() {
        let v = list_tools();
        let names: Vec<&str> = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(
            names.contains(&"detect_fields"),
            "detect_fields should be in the tool list"
        );
    }

    #[test]
    fn all_tools_have_input_schema() {
        let v = list_tools();
        let tools = v["tools"].as_array().unwrap();
        for tool in tools {
            let name = tool["name"].as_str().unwrap_or("<unnamed>");
            assert!(
                tool.get("inputSchema").is_some(),
                "tool '{name}' should have an inputSchema"
            );
            assert!(
                tool["inputSchema"].is_object(),
                "tool '{name}' inputSchema should be an object"
            );
        }
    }

    #[test]
    fn check_billing_status_in_tool_list() {
        let v = list_tools();
        let names: Vec<&str> = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(
            names.contains(&"check_billing_status"),
            "check_billing_status should be in the tool list"
        );
    }

    #[test]
    fn purchase_credits_in_tool_list() {
        let v = list_tools();
        let names: Vec<&str> = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(
            names.contains(&"purchase_credits"),
            "purchase_credits should be in the tool list"
        );
    }

    #[test]
    fn purchase_credits_has_enum_schema() {
        let v = list_tools();
        let tools = v["tools"].as_array().unwrap();
        let tool = tools
            .iter()
            .find(|t| t["name"].as_str() == Some("purchase_credits"))
            .expect("purchase_credits tool should exist");
        let credits_prop = &tool["inputSchema"]["properties"]["credits"];
        assert!(
            credits_prop.get("enum").is_some(),
            "purchase_credits.credits should have an enum constraint"
        );
        let enum_vals: Vec<i64> = credits_prop["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_i64())
            .collect();
        assert_eq!(enum_vals, vec![10, 50, 200]);
    }
}

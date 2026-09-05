//! MCP resources — URI-addressable read-only views into blocksign state.

use serde_json::{json, Value};

use crate::api_client::ApiClient;

/// Resource catalog returned by `resources/list`.
pub fn list_resources() -> Value {
    json!({
        "resources": [
            {
                "uri": "templates://catalog",
                "name": "Template catalog",
                "description": "All contract templates visible to the calling org.",
                "mimeType": "application/json"
            }
        ],
        "resourceTemplates": [
            {
                "uriTemplate": "agreement://{id}",
                "name": "Agreement state",
                "description": "Live state of a specific agreement (envelope + signers).",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "template://{slug}",
                "name": "Template definition",
                "description": "Markdown body + variables schema for a template slug.",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "verification://{id}",
                "name": "Verification result",
                "description": "Public audit-chain verification for an agreement.",
                "mimeType": "application/json"
            }
        ]
    })
}

/// Read a resource by URI.
pub async fn read_resource(params: Value, auth: &str, api: &ApiClient) -> Result<Value, String> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing resource uri".to_string())?;

    let (path, mime) = if uri == "templates://catalog" {
        ("/v1/templates".to_string(), "application/json")
    } else if let Some(id) = uri.strip_prefix("agreement://") {
        (format!("/v1/agent/agreements/{id}"), "application/json")
    } else if let Some(slug) = uri.strip_prefix("template://") {
        (format!("/v1/templates/{slug}"), "application/json")
    } else if let Some(id) = uri.strip_prefix("verification://") {
        (format!("/v1/verify/{id}"), "application/json")
    } else {
        return Err(format!("unknown resource uri: {uri}"));
    };

    let body: Value = api.get(&path, auth).await.map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string());

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": mime,
            "text": text
        }]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_resources_includes_all_uri_templates() {
        let v = list_resources();
        let templates: Vec<&str> = v["resourceTemplates"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["uriTemplate"].as_str())
            .collect();
        assert!(templates.contains(&"agreement://{id}"));
        assert!(templates.contains(&"template://{slug}"));
        assert!(templates.contains(&"verification://{id}"));
    }

    #[test]
    fn list_resources_has_static_catalog_entry() {
        let v = list_resources();
        let resources: Vec<&str> = v["resources"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["uri"].as_str())
            .collect();
        assert!(resources.contains(&"templates://catalog"));
    }

    #[tokio::test]
    async fn read_unknown_uri_returns_error() {
        let api = ApiClient::new("http://nowhere");
        let result = read_resource(json!({"uri": "foo://bar"}), "Bearer x", &api).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown resource"));
    }

    #[tokio::test]
    async fn missing_uri_returns_error() {
        let api = ApiClient::new("http://nowhere");
        let result = read_resource(json!({}), "Bearer x", &api).await;
        assert!(result.is_err());
    }

    #[test]
    fn all_resources_have_mime_type() {
        let v = list_resources();

        // Check static resources
        let resources = v["resources"].as_array().unwrap();
        for r in resources {
            let uri = r["uri"].as_str().unwrap_or("<no uri>");
            assert!(
                r.get("mimeType").is_some() && r["mimeType"].is_string(),
                "resource '{uri}' should have a mimeType string"
            );
        }

        // Check resource templates
        let templates = v["resourceTemplates"].as_array().unwrap();
        for t in templates {
            let uri_template = t["uriTemplate"].as_str().unwrap_or("<no uriTemplate>");
            assert!(
                t.get("mimeType").is_some() && t["mimeType"].is_string(),
                "resourceTemplate '{uri_template}' should have a mimeType string"
            );
        }
    }
}

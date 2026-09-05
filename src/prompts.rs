//! MCP prompts — guided workflows the client can offer the user.

use serde_json::{json, Value};

/// Catalog returned by `prompts/list`.
pub fn list_prompts() -> Value {
    json!({
        "prompts": [
            {
                "name": "create_nda",
                "description": "Guide the user through creating a mutual NDA between two parties.",
                "arguments": [
                    { "name": "party_a", "description": "First party name + email", "required": true },
                    { "name": "party_b", "description": "Second party name + email", "required": true },
                    { "name": "jurisdiction", "description": "Governing jurisdiction (e.g. Delaware)", "required": false }
                ]
            },
            {
                "name": "review_status",
                "description": "Review the status of an agreement and recommend next actions.",
                "arguments": [
                    { "name": "agreement_id", "description": "Agreement UUID", "required": true }
                ]
            },
            {
                "name": "explain_verification",
                "description": "Explain how blocksign.red's blockchain anchoring verifies an agreement's integrity.",
                "arguments": []
            }
        ]
    })
}

/// Render a prompt by name with its arguments interpolated.
pub fn get_prompt(params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing prompt name".to_string())?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let messages = match name {
        "create_nda" => create_nda_messages(&args),
        "review_status" => review_status_messages(&args),
        "explain_verification" => explain_verification_messages(),
        other => return Err(format!("unknown prompt: {other}")),
    };

    Ok(json!({
        "description": format!("blocksign.red prompt: {name}"),
        "messages": messages
    }))
}

fn create_nda_messages(args: &Value) -> Value {
    let party_a = args.get("party_a").and_then(Value::as_str).unwrap_or("Party A");
    let party_b = args.get("party_b").and_then(Value::as_str).unwrap_or("Party B");
    let jurisdiction = args
        .get("jurisdiction")
        .and_then(Value::as_str)
        .unwrap_or("Delaware");

    json!([
        {
            "role": "user",
            "content": {
                "type": "text",
                "text": format!(
                    "Create a mutual NDA between {party_a} and {party_b}, governed by {jurisdiction} law. Use the `create_agreement` tool with template='nda' and the appropriate variables. After sending, return the verification URL and signing links so I can share them."
                )
            }
        }
    ])
}

fn review_status_messages(args: &Value) -> Value {
    let id = args
        .get("agreement_id")
        .and_then(Value::as_str)
        .unwrap_or("<agreement_id>");
    json!([
        {
            "role": "user",
            "content": {
                "type": "text",
                "text": format!(
                    "Call `get_agreement_status` for agreement_id={id}. Summarize: which signers have signed, who is pending, and what action (if any) the requester should take next."
                )
            }
        }
    ])
}

fn explain_verification_messages() -> Value {
    json!([
        {
            "role": "user",
            "content": {
                "type": "text",
                "text": "Explain how blocksign.red provides tamper-evident verification: SHA-256 hash chain over signing events, Merkle root anchored to Bitcoin and Base (Ethereum L2), public /v1/verify endpoint requires no auth. Mention that anyone can verify by URL alone."
            }
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_prompts_returns_all_three() {
        let v = list_prompts();
        let names: Vec<&str> = v["prompts"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p["name"].as_str())
            .collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"create_nda"));
        assert!(names.contains(&"review_status"));
        assert!(names.contains(&"explain_verification"));
    }

    #[test]
    fn get_prompt_create_nda_interpolates_args() {
        let result = get_prompt(json!({
            "name": "create_nda",
            "arguments": {
                "party_a": "Acme Corp",
                "party_b": "Bob Smith",
                "jurisdiction": "California"
            }
        })).unwrap();
        let text = result["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("Acme Corp"));
        assert!(text.contains("Bob Smith"));
        assert!(text.contains("California"));
    }

    #[test]
    fn get_prompt_unknown_returns_error() {
        let result = get_prompt(json!({"name": "no_such_prompt"}));
        assert!(result.is_err());
    }

    #[test]
    fn get_prompt_missing_name_returns_error() {
        let result = get_prompt(json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn explain_verification_mentions_blockchain() {
        let result = get_prompt(json!({"name": "explain_verification"})).unwrap();
        let text = result["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.to_lowercase().contains("bitcoin"));
        assert!(text.to_lowercase().contains("merkle"));
    }

    #[test]
    fn all_prompts_have_description() {
        let v = list_prompts();
        let prompts = v["prompts"].as_array().unwrap();
        for p in prompts {
            let name = p["name"].as_str().unwrap_or("<unnamed>");
            assert!(
                p.get("description").is_some() && p["description"].is_string(),
                "prompt '{name}' should have a description"
            );
            assert!(
                !p["description"].as_str().unwrap().is_empty(),
                "prompt '{name}' description should not be empty"
            );
        }
    }

    #[test]
    fn get_prompt_review_status_interpolates_id() {
        let result = get_prompt(json!({
            "name": "review_status",
            "arguments": {
                "agreement_id": "550e8400-e29b-41d4-a716-446655440000"
            }
        }))
        .unwrap();
        let text = result["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(
            text.contains("550e8400-e29b-41d4-a716-446655440000"),
            "review_status prompt should interpolate the agreement_id, got: {text}"
        );
    }
}

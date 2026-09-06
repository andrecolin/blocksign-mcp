//! Thin HTTP wrapper around the blocksign REST API.
//!
//! Every MCP call goes through here. The caller's `Authorization` header is
//! forwarded as-is — the API is the source of truth for auth/capability checks.

use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API returned {status}: {body}")]
    Status { status: u16, body: String },
    #[error("missing authorization header")]
    MissingAuth,
}

#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base: String,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("blocksign-mcp/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            http,
            base: base_url.into(),
        }
    }

    /// GET path, returning JSON parsed into T.
    pub async fn get<T: DeserializeOwned>(&self, path: &str, auth: &str) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .http
            .get(&url)
            .header("authorization", auth)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// POST path with JSON body, returning JSON parsed into T.
    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        auth: &str,
        body: &serde_json::Value,
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .http
            .post(&url)
            .header("authorization", auth)
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// POST with JSON body and NO Authorization header — for the keyless
    /// guest endpoint (`/v1/public/agreements`) where payment is the identity.
    pub async fn post_public<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// POST with no body (used for /void).
    pub async fn post_empty<T: DeserializeOwned>(
        &self,
        path: &str,
        auth: &str,
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .http
            .post(&url)
            .header("authorization", auth)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// POST multipart/form-data — used for document upload with text content.
    /// `text_fields`: extra form text fields (name, value).
    /// `file_content`: bytes of the file part.
    pub async fn post_multipart<T: DeserializeOwned>(
        &self,
        path: &str,
        auth: &str,
        text_fields: &[(&str, &str)],
        file_content: Vec<u8>,
        file_name: &str,
        content_type: &str,
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base, path);

        let file_part = reqwest::multipart::Part::bytes(file_content)
            .file_name(file_name.to_owned())
            .mime_str(content_type)
            .map_err(ApiError::Http)?;

        let mut form = reqwest::multipart::Form::new().part("file", file_part);
        for (name, value) in text_fields {
            form = form.text(name.to_string(), value.to_string());
        }

        let resp = self
            .http
            .post(&url)
            .header("authorization", auth)
            .multipart(form)
            .send()
            .await?;
        Self::parse(resp).await
    }

    async fn parse<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, ApiError> {
        let status = resp.status();
        if status.is_success() {
            let value = resp.json::<T>().await?;
            Ok(value)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ApiError::Status {
                status: status.as_u16(),
                body,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs_with_base_url() {
        let c = ApiClient::new("http://localhost:8080");
        assert!(c.base.starts_with("http://"));
    }
}

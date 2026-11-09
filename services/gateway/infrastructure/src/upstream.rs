//! Reqwest-backed upstream client.
//!
//! One attempt per call; the application applies the bounded retry policy. The
//! client forwards the user JWT unchanged, sets the correlation id, strips
//! hop-by-hop headers, and refuses redirects.

use std::time::Duration;

use async_trait::async_trait;
use gateway_application::ports::{ForwardRequest, UpstreamClient, UpstreamError, UpstreamResponse};
use reqwest::header::{HeaderName, HeaderValue, CONTENT_TYPE};

/// Forwards requests to the owning service over internal HTTP.
pub struct ReqwestUpstreamClient {
    client: reqwest::Client,
    base_url: String,
}

impl ReqwestUpstreamClient {
    /// Creates the client for one service base URL.
    #[must_use]
    pub fn new(base_url: impl Into<String>, timeout_ms: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
            | "content-length"
            | "x-correlation-id"
    )
}

#[async_trait]
impl UpstreamClient for ReqwestUpstreamClient {
    async fn forward(&self, request: ForwardRequest) -> Result<UpstreamResponse, UpstreamError> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| UpstreamError::Rejected(error.to_string()))?;
        let url = format!("{}{}", self.base_url, request.path_and_query);
        let mut builder = self.client.request(method, url);

        // Forward the support console's X-Support-Reason header unchanged. It
        // must never be logged: the gateway records ids, outcomes, and the
        // correlation id, never request headers.
        for (name, value) in &request.headers {
            if is_hop_by_hop(name) {
                continue;
            }
            let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) else {
                continue;
            };
            builder = builder.header(name, value);
        }
        builder = builder.header("x-correlation-id", &request.correlation_id);

        let response = builder.body(request.body).send().await.map_err(|error| {
            if error.is_timeout() {
                UpstreamError::Timeout
            } else {
                UpstreamError::Unavailable(error.to_string())
            }
        })?;

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_string(), value.to_string()))
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|error| UpstreamError::Unavailable(error.to_string()))?
            .to_vec();

        Ok(UpstreamResponse {
            status,
            headers,
            body,
            content_type,
        })
    }
}

//! The private `identity-exports` Supabase Storage adapter.
//!
//! The bucket is private and encrypted at rest; the only read path is a signed
//! URL that expires at `downloadExpiresAt` (24 hours). The worker deletes the
//! object after the window.

use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::Duration;
use identity_application::ports::{ExportStore, ExternalError};

use crate::http_retry::{is_transient, pause_before_retry};

/// Calls Supabase Storage over HTTPS.
pub struct SupabaseStorage {
    client: reqwest::Client,
    base_url: String,
    service_role_key: String,
    bucket: String,
}

impl SupabaseStorage {
    /// Creates the adapter.
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        service_role_key: impl Into<String>,
        bucket: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(StdDuration::from_secs(1))
            .timeout(StdDuration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_role_key: service_role_key.into(),
            bucket: bucket.into(),
        }
    }

    fn object_url(&self, key: &str) -> String {
        format!(
            "{}/object/{}/{}",
            self.base_url.trim_end_matches('/'),
            self.bucket,
            key
        )
    }

    fn sign_url(&self, key: &str) -> String {
        format!(
            "{}/object/sign/{}/{}",
            self.base_url.trim_end_matches('/'),
            self.bucket,
            key
        )
    }
}

#[async_trait]
impl ExportStore for SupabaseStorage {
    async fn put(&self, key: &str, body: Vec<u8>) -> Result<(), ExternalError> {
        let url = self.object_url(key);
        for attempt in 0..3u32 {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.service_role_key)
                .header("x-upsert", "true")
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.clone())
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "storage refused the upload with {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    tracing::warn!(%error, "storage upload failed; retrying");
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "storage is unreachable: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(
            "storage did not answer the upload".to_string(),
        ))
    }

    async fn signed_url(&self, key: &str, valid_for: Duration) -> Result<String, ExternalError> {
        let url = self.sign_url(key);
        let seconds = valid_for.num_seconds();
        for attempt in 0..3u32 {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.service_role_key)
                .json(&serde_json::json!({ "expiresIn": seconds }))
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    let value = response
                        .json::<serde_json::Value>()
                        .await
                        .map_err(|error| {
                            ExternalError::Refused(format!(
                                "storage returned an unreadable signed link: {error}"
                            ))
                        })?;
                    let signed = value
                        .get("signedURL")
                        .or_else(|| value.get("signedUrl"))
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            ExternalError::Refused("storage returned no signed link".to_string())
                        })?;
                    if signed.starts_with('/') {
                        return Ok(format!("{}{}", self.base_url.trim_end_matches('/'), signed));
                    }
                    return Ok(signed.to_string());
                }
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "storage refused the signed link with {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    tracing::warn!(%error, "signed link call failed; retrying");
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "storage is unreachable: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(
            "storage did not answer the signed-link call".to_string(),
        ))
    }

    async fn delete(&self, key: &str) -> Result<(), ExternalError> {
        let url = self.object_url(key);
        for attempt in 0..3u32 {
            let response = self
                .client
                .delete(&url)
                .bearer_auth(&self.service_role_key)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "storage refused the delete with {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    tracing::warn!(%error, "storage delete failed; retrying");
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "storage is unreachable: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(
            "storage did not answer the delete".to_string(),
        ))
    }
}

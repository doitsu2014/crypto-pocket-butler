//! The Supabase Auth admin adapter: session revocation and user deletion.
//!
//! The exact admin route for a single-session logout is the provider's to
//! define; this adapter is the only place that would change if the project's
//! Auth version exposes a user-level logout only.

use std::time::Duration;

use async_trait::async_trait;
use identity_application::ports::{AuthAdmin, ExternalError};

use crate::http_retry::{is_transient, pause_before_retry};

/// Calls the Supabase Auth admin API with the service-role key.
pub struct HttpAuthAdmin {
    client: reqwest::Client,
    base_url: String,
    service_role_key: String,
}

impl HttpAuthAdmin {
    /// Creates the adapter.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_role_key: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_role_key: service_role_key.into(),
        }
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<(), ExternalError> {
        for attempt in 0..3u32 {
            let response = request
                .try_clone()
                .ok_or_else(|| {
                    ExternalError::Unavailable(format!("{operation} request cannot be replayed"))
                })?
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
                        "supabase auth refused {operation} with {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    tracing::warn!(%error, operation, "supabase auth call failed; retrying");
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "supabase auth is unreachable for {operation}: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "supabase auth did not answer {operation}"
        )))
    }
}

#[async_trait]
impl AuthAdmin for HttpAuthAdmin {
    async fn revoke_session(
        &self,
        auth_subject: &str,
        session_id: &str,
    ) -> Result<(), ExternalError> {
        let url = format!(
            "{}/admin/users/{auth_subject}/sessions/{session_id}/logout",
            self.base_url.trim_end_matches('/')
        );
        self.send(self.client.post(url), "session revoke").await
    }

    async fn delete_user(&self, auth_subject: &str) -> Result<(), ExternalError> {
        let url = format!(
            "{}/admin/users/{auth_subject}",
            self.base_url.trim_end_matches('/')
        );
        self.send(self.client.delete(url), "user delete").await
    }
}

//! The Supabase Auth admin read and write of `app_metadata.role` (BR2.10).
//!
//! One attempt each: a duplicate privileged change is worse than a surfaced
//! failure, so this adapter never retries. The service-role key comes from a
//! secret file.

use std::time::Duration;

use async_trait::async_trait;
use identity_application::authorisation::ports::{RoleReader, RoleWriter};
use identity_application::ports::ExternalError;
use identity_domain::Role;
use serde_json::json;

/// The Supabase Auth admin client.
pub struct SupabaseRoleWriter {
    client: reqwest::Client,
    base_url: String,
    service_role_key: String,
}

impl SupabaseRoleWriter {
    /// Creates the writer.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_role_key: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            service_role_key: service_role_key.into(),
        }
    }
}

#[async_trait]
impl RoleWriter for SupabaseRoleWriter {
    async fn set_role(&self, subject: &str, role: Role) -> Result<(), ExternalError> {
        // Read first: the provider's Admin API replaces `app_metadata` with the
        // object it is given, so a blind `{"role": ...}` write would erase
        // `has_exchange_credentials` (which drives the MFA requirement) and any
        // other provider metadata. Merge the role into what is already there.
        let mut app_metadata = self.read_app_metadata(subject).await?;
        app_metadata["role"] = json!(role.provider_value());

        let url = format!("{}/admin/users/{subject}", self.base_url);
        let payload = json!({ "app_metadata": app_metadata });

        match self
            .client
            .put(&url)
            .bearer_auth(&self.service_role_key)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => Ok(()),
            Ok(response) if response.status().is_client_error() => {
                Err(ExternalError::Refused(format!(
                    "supabase auth refused the role change with {}",
                    response.status()
                )))
            }
            Ok(response) => Err(ExternalError::Unavailable(format!(
                "supabase auth answered {}",
                response.status()
            ))),
            Err(error) => Err(ExternalError::Unavailable(format!(
                "supabase auth is unreachable: {error}"
            ))),
        }
    }
}

#[async_trait]
impl RoleReader for SupabaseRoleWriter {
    async fn read_role(&self, subject: &str) -> Result<Role, ExternalError> {
        let app_metadata = self.read_app_metadata(subject).await?;
        // The same mapping the claim path uses: only `admin` and `support` are
        // recognised, and a missing or unknown value is the regular user.
        Ok(Role::from_claim(
            app_metadata.get("role").and_then(serde_json::Value::as_str),
        ))
    }
}

impl SupabaseRoleWriter {
    /// Reads the target user's current `app_metadata`.
    async fn read_app_metadata(&self, subject: &str) -> Result<serde_json::Value, ExternalError> {
        let url = format!("{}/admin/users/{subject}", self.base_url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.service_role_key)
            .send()
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("supabase auth is unreachable: {error}"))
            })?;

        if response.status().is_client_error() {
            return Err(ExternalError::Refused(format!(
                "supabase auth refused the metadata read with {}",
                response.status()
            )));
        }
        if !response.status().is_success() {
            return Err(ExternalError::Unavailable(format!(
                "supabase auth answered {} on the metadata read",
                response.status()
            )));
        }

        let body: serde_json::Value = response.json().await.map_err(|error| {
            ExternalError::Unavailable(format!("the user record is not JSON: {error}"))
        })?;
        match body.get("app_metadata") {
            Some(metadata) if metadata.is_object() => Ok(metadata.clone()),
            Some(_) => Err(ExternalError::Unavailable(
                "the user record's app_metadata is not an object".to_string(),
            )),
            None => Ok(json!({})),
        }
    }
}

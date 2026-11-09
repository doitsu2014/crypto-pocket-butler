//! The Account/Ledger/Portfolio privacy adapters (C5/C7/C8 change requests).
//!
//! Each call is synchronous internal HTTP with a service token, a 30-second
//! budget, and at most three attempts with jitter. The anonymise call carries an
//! idempotency key so a retry applies once.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use identity_application::ports::{ContributorGateway, ExternalError};
use identity_domain::{Contributor, RetainedCategory, UserId};

use crate::http_retry::{is_transient, pause_before_retry};

/// Calls the contributing services' privacy operations.
pub struct HttpContributors {
    client: reqwest::Client,
    endpoints: BTreeMap<Contributor, String>,
    service_token: String,
}

impl HttpContributors {
    /// Creates the adapter from each contributor's base URL.
    #[must_use]
    pub fn new(
        account_url: impl Into<String>,
        ledger_url: impl Into<String>,
        portfolio_url: impl Into<String>,
        service_token: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let endpoints = [
            (Contributor::Account, account_url.into()),
            (Contributor::Ledger, ledger_url.into()),
            (Contributor::Portfolio, portfolio_url.into()),
        ]
        .into_iter()
        .collect();
        Self {
            client,
            endpoints,
            service_token: service_token.into(),
        }
    }

    fn base(&self, contributor: Contributor) -> Result<&str, ExternalError> {
        self.endpoints
            .get(&contributor)
            .map(String::as_str)
            .ok_or_else(|| {
                ExternalError::Unavailable(format!(
                    "no endpoint is configured for {}",
                    contributor.as_str()
                ))
            })
    }
}

fn parse_retained(value: &serde_json::Value) -> Result<Vec<RetainedCategory>, ExternalError> {
    let Some(array) = value
        .get("retainedCategories")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut categories = Vec::with_capacity(array.len());
    for entry in array {
        let category = entry
            .get("category")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ExternalError::Refused("a retained category has no label".to_string())
            })?;
        let basis = entry
            .get("basis")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ExternalError::Refused("a retained category has no legal basis".to_string())
            })?;
        categories.push(RetainedCategory::new(category, basis).map_err(|error| {
            ExternalError::Refused(format!("a retained category is invalid: {error}"))
        })?);
    }
    Ok(categories)
}

#[async_trait]
impl ContributorGateway for HttpContributors {
    async fn export(
        &self,
        contributor: Contributor,
        user_id: UserId,
    ) -> Result<serde_json::Value, ExternalError> {
        let url = format!(
            "{}/v1/internal/privacy/export?userId={user_id}",
            self.base(contributor)?.trim_end_matches('/')
        );
        for attempt in 0..3u32 {
            let response = self
                .client
                .get(&url)
                .bearer_auth(&self.service_token)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    return response.json::<serde_json::Value>().await.map_err(|error| {
                        ExternalError::Refused(format!(
                            "{} returned an unreadable export: {error}",
                            contributor.as_str()
                        ))
                    });
                }
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "{} refused the export with {}",
                        contributor.as_str(),
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    tracing::warn!(%error, contributor = contributor.as_str(), "export call failed");
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "{} is unreachable: {error}",
                        contributor.as_str()
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "{} did not answer the export",
            contributor.as_str()
        )))
    }

    async fn anonymise(
        &self,
        contributor: Contributor,
        user_id: UserId,
        idempotency_key: &str,
    ) -> Result<Vec<RetainedCategory>, ExternalError> {
        let url = format!(
            "{}/v1/internal/privacy/anonymise",
            self.base(contributor)?.trim_end_matches('/')
        );
        let body = serde_json::json!({ "userId": user_id.to_string() });
        for attempt in 0..3u32 {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.service_token)
                .header("idempotency-key", idempotency_key)
                .json(&body)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    let value = response
                        .json::<serde_json::Value>()
                        .await
                        .unwrap_or_else(|_| serde_json::json!({}));
                    return parse_retained(&value);
                }
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "{} refused the anonymise with {}",
                        contributor.as_str(),
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    tracing::warn!(%error, contributor = contributor.as_str(), "anonymise call failed");
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "{} is unreachable: {error}",
                        contributor.as_str()
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "{} did not answer the anonymise",
            contributor.as_str()
        )))
    }
}

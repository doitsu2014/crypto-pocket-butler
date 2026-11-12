//! The Account and Portfolio support-metadata clients (BR3.9, BR3.10, C1/C2).
//!
//! Each owner exposes one internal endpoint that returns only the allowlisted
//! summary fields. Identity asks for exactly that shape and nothing else, so a
//! forgotten filter cannot leak a holding, a balance, or an address. The calls
//! are one attempt inside a short timeout; the application layer fans them out
//! under its own 500 ms deadline and refuses the read when either misses it.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{
    AccountSummary, ExternalError, PortfolioSummary, SupportAccountsSource, SupportPortfoliosSource,
};
use identity_domain::UserId;
use serde_json::Value;

/// Per-request budget for one owner. It sits above the owner's p99 so a slow
/// but healthy call is not turned into a refusal by the client alone.
const REQUEST_TIMEOUT: Duration = Duration::from_millis(120);

/// The Account service's allowlisted projection.
pub struct HttpSupportAccounts {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpSupportAccounts {
    /// Creates the client from the Account base URL and Identity's own token.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        Self {
            client: build_client(),
            base_url: base_url.into(),
            service_token: service_token.into(),
        }
    }
}

#[async_trait]
impl SupportAccountsSource for HttpSupportAccounts {
    async fn accounts(&self, user_id: UserId) -> Result<Vec<AccountSummary>, ExternalError> {
        let url = format!(
            "{}/v1/internal/support-metadata/{user_id}",
            self.base_url.trim_end_matches('/')
        );
        let value = get_json(&self.client, &url, &self.service_token, "account").await?;
        parse_accounts(&value)
    }
}

/// The Portfolio service's allowlisted projection.
pub struct HttpSupportPortfolios {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpSupportPortfolios {
    /// Creates the client from the Portfolio base URL and Identity's own token.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        Self {
            client: build_client(),
            base_url: base_url.into(),
            service_token: service_token.into(),
        }
    }
}

#[async_trait]
impl SupportPortfoliosSource for HttpSupportPortfolios {
    async fn portfolios(&self, user_id: UserId) -> Result<Vec<PortfolioSummary>, ExternalError> {
        let url = format!(
            "{}/v1/internal/support-metadata/{user_id}",
            self.base_url.trim_end_matches('/')
        );
        let value = get_json(&self.client, &url, &self.service_token, "portfolio").await?;
        parse_portfolios(&value)
    }
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(1))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

async fn get_json(
    client: &reqwest::Client,
    url: &str,
    service_token: &str,
    owner: &str,
) -> Result<Value, ExternalError> {
    let response = client
        .get(url)
        .bearer_auth(service_token)
        .send()
        .await
        .map_err(|error| {
            ExternalError::Unavailable(format!("{owner} could not be reached: {error}"))
        })?;
    if !response.status().is_success() {
        return Err(ExternalError::Unavailable(format!(
            "{owner} answered {}",
            response.status().as_u16()
        )));
    }
    response.json::<Value>().await.map_err(|error| {
        ExternalError::Refused(format!(
            "{owner} returned an unreadable projection: {error}"
        ))
    })
}

fn parse_accounts(value: &Value) -> Result<Vec<AccountSummary>, ExternalError> {
    let Some(array) = value.get("accounts").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut accounts = Vec::with_capacity(array.len());
    for entry in array {
        accounts.push(AccountSummary {
            id: parse_uuid(entry, "id")?,
            label: required_str(entry, "label")?,
            source_type: required_str(entry, "sourceType")?,
            sync_status: required_str(entry, "syncStatus")?,
            last_synced_at: parse_timestamp(entry.get("lastSyncedAt"))?,
        });
    }
    Ok(accounts)
}

fn parse_portfolios(value: &Value) -> Result<Vec<PortfolioSummary>, ExternalError> {
    let Some(array) = value.get("portfolios").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut portfolios = Vec::with_capacity(array.len());
    for entry in array {
        portfolios.push(PortfolioSummary {
            id: parse_uuid(entry, "id")?,
            label: required_str(entry, "label")?,
        });
    }
    Ok(portfolios)
}

fn parse_uuid(entry: &Value, field: &str) -> Result<uuid::Uuid, ExternalError> {
    let raw = required_str(entry, field)?;
    uuid::Uuid::parse_str(&raw)
        .map_err(|_| ExternalError::Refused(format!("a projection {field} is not a UUID")))
}

fn required_str(entry: &Value, field: &str) -> Result<String, ExternalError> {
    entry
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ExternalError::Refused(format!("a projection is missing {field}")))
}

fn parse_timestamp(value: Option<&Value>) -> Result<Option<DateTime<Utc>>, ExternalError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let raw = value.as_str().ok_or_else(|| {
        ExternalError::Refused("a projection timestamp is not a string".to_string())
    })?;
    DateTime::parse_from_rfc3339(raw)
        .map(|at| Some(at.with_timezone(&Utc)))
        .map_err(|_| ExternalError::Refused("a projection timestamp is not RFC 3339".to_string()))
}

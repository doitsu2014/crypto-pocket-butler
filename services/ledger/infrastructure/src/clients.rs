//! HTTP clients for C5 (Account), C4 (Asset Registry), and C10 (Audit).
//!
//! C5 reads use bounded retry (1 s / 3 s / 10 s, three attempts); a failure
//! degrades ingest only. C4 resolution fails the write with a problem document.
//! C10 appends use a 1 s connect and 5 s total timeout; a failure is retried by
//! the trail writer and never blocks the committed row.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{
    AccountClient, AssetResolver, AuditClient, AuditRecord, ExternalError, FxQuote, Holding,
    MarketDataClient, PriceQuote,
};
use ledger_domain::{IncomingTransaction, Kind, Source};
use reqwest::{Response, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// The C5 retry delays.
pub const C5_RETRY_SECONDS: [u64; 3] = [1, 3, 10];
/// The C5 per-attempt timeout.
pub const C5_TIMEOUT: Duration = Duration::from_secs(10);
/// The C4 per-attempt timeout.
pub const C4_TIMEOUT: Duration = Duration::from_secs(3);
/// The C4 list cache window.
pub const C4_CACHE: Duration = Duration::from_secs(300);
/// The C10 connect and total timeouts.
pub const AUDIT_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
pub const AUDIT_TIMEOUT: Duration = Duration::from_secs(5);

fn classify(response: Response, dependency: &str) -> Result<Response, ExternalError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if status.is_client_error() {
        return Err(ExternalError::Refused(format!(
            "{dependency} returned {status}"
        )));
    }
    Err(ExternalError::Unavailable(format!(
        "{dependency} returned {status}"
    )))
}

async fn with_retry<F, Fut>(dependency: &str, mut attempt: F) -> Result<Response, ExternalError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Response, reqwest::Error>>,
{
    let mut last = None;
    for (index, delay) in C5_RETRY_SECONDS.iter().enumerate() {
        match attempt().await {
            Ok(response) => match classify(response, dependency) {
                Ok(response) => return Ok(response),
                Err(error @ ExternalError::Refused(_)) => return Err(error),
                Err(error) => last = Some(error),
            },
            Err(error) => last = Some(ExternalError::Unavailable(format!("{dependency}: {error}"))),
        }
        if index + 1 < C5_RETRY_SECONDS.len() {
            tokio::time::sleep(Duration::from_secs(*delay)).await;
        }
    }
    Err(last.unwrap_or_else(|| {
        ExternalError::Unavailable(format!("{dependency}: no attempt was made"))
    }))
}

/// Reads Account's venue and imported transactions through C5.
pub struct HttpAccountClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpAccountClient {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(C5_TIMEOUT)
            .connect_timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HoldingRow {
    account_id: Uuid,
    #[serde(default)]
    asset_id: Option<String>,
    #[serde(default)]
    quantity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VenueTransaction {
    #[serde(default)]
    external_id: Option<String>,
    #[serde(default)]
    asset_id: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    quantity: Option<String>,
    #[serde(default)]
    price: Option<String>,
    #[serde(default)]
    executed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    source: Option<String>,
}

impl VenueTransaction {
    fn into_incoming(self, account_id: Uuid) -> IncomingTransaction {
        let kind = self
            .kind
            .as_deref()
            .map(Kind::parse)
            .transpose()
            .ok()
            .flatten();
        let quantity = self
            .quantity
            .as_deref()
            .and_then(|value| value.parse::<Decimal>().ok());
        let price = self
            .price
            .as_deref()
            .and_then(|value| value.parse::<Decimal>().ok());
        let source = self
            .source
            .as_deref()
            .map(Source::parse)
            .transpose()
            .ok()
            .flatten()
            .unwrap_or(Source::Venue);
        let line_error = if kind.is_none() {
            Some("kind is missing or unknown".to_string())
        } else if quantity.is_none() {
            Some("quantity is missing or not a decimal".to_string())
        } else if self.executed_at.is_none() {
            Some("executedAt is missing".to_string())
        } else {
            None
        };
        IncomingTransaction {
            account_id: Some(account_id),
            asset_id: self.asset_id.unwrap_or_default(),
            kind,
            quantity,
            price,
            currency: None,
            executed_at: self.executed_at,
            external_ref: self.external_id,
            source,
            line_error,
        }
    }
}

impl HttpAccountClient {
    async fn holdings(&self, user_id: &str) -> Result<Vec<HoldingRow>, ExternalError> {
        let holdings_url = format!("{}/v1/users/{user_id}/holdings", self.base_url);
        with_retry("C5 getHoldings", || {
            self.client
                .get(&holdings_url)
                .bearer_auth(&self.token)
                .send()
        })
        .await?
        .error_for_status()
        .map_err(|error| ExternalError::Unavailable(format!("C5 holdings body: {error}")))?
        .json::<Vec<HoldingRow>>()
        .await
        .map_err(|error| ExternalError::Unavailable(format!("C5 holdings body: {error}")))
    }
}

#[async_trait]
impl AccountClient for HttpAccountClient {
    async fn transactions_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<IncomingTransaction>, ExternalError> {
        let holdings = self.holdings(user_id).await?;
        let account_ids: BTreeSet<Uuid> = holdings.into_iter().map(|row| row.account_id).collect();
        let mut rows = Vec::new();
        for account_id in account_ids {
            let url = format!("{}/v1/accounts/{account_id}/transactions", self.base_url);
            let transactions = with_retry("C5 getAccountTransactions", || {
                self.client.get(&url).bearer_auth(&self.token).send()
            })
            .await?
            .error_for_status()
            .map_err(|error| ExternalError::Unavailable(format!("C5 transactions body: {error}")))?
            .json::<Vec<VenueTransaction>>()
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("C5 transactions body: {error}"))
            })?;
            rows.extend(
                transactions
                    .into_iter()
                    .map(|row| row.into_incoming(account_id)),
            );
        }
        Ok(rows)
    }

    async fn holdings_for_user(&self, user_id: &str) -> Result<Vec<Holding>, ExternalError> {
        Ok(self
            .holdings(user_id)
            .await?
            .into_iter()
            .filter_map(|row| {
                let asset_id = row.asset_id.filter(|asset| !asset.trim().is_empty())?;
                let quantity = row.quantity?.parse::<Decimal>().ok()?;
                Some(Holding {
                    account_id: row.account_id,
                    asset_id: Some(asset_id),
                    quantity,
                })
            })
            .collect())
    }
}

/// Resolves an asset through C4's canonical asset list, cached for five minutes.
pub struct HttpAssetResolver {
    client: reqwest::Client,
    base_url: String,
    token: String,
    cache: RwLock<Option<(Instant, Arc<BTreeSet<String>>)>>,
}

impl HttpAssetResolver {
    /// Creates the resolver.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(C4_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
            cache: RwLock::new(None),
        }
    }

    async fn canonical_ids(&self) -> Result<Arc<BTreeSet<String>>, ExternalError> {
        if let Some((fetched_at, ids)) = self.cache.read().await.clone() {
            if fetched_at.elapsed() < C4_CACHE {
                return Ok(ids);
            }
        }
        let url = format!("{}/v1/assets?limit=10000", self.base_url);
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C4 listAssets: {error}")))?;
        let response = classify(response, "C4 listAssets")?;
        let assets = response
            .json::<Vec<AssetDto>>()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C4 listAssets body: {error}")))?;
        let ids: BTreeSet<String> = assets.into_iter().map(|asset| asset.id).collect();
        let ids = Arc::new(ids);
        *self.cache.write().await = Some((Instant::now(), ids.clone()));
        Ok(ids)
    }
}

#[derive(Debug, Deserialize)]
struct AssetDto {
    id: String,
}

#[async_trait]
impl AssetResolver for HttpAssetResolver {
    async fn exists(&self, asset_id: &str) -> Result<bool, ExternalError> {
        Ok(self.canonical_ids().await?.contains(asset_id))
    }
}

/// The C6 read-path budget: one attempt, 150 ms total. C6 p95 is assumed to be
/// at most 120 ms, leaving 30 ms of headroom inside the 400 ms `getPnl` budget
/// (NFR2.1). A timeout degrades the read to labelled unpriced marks; it never
/// becomes a 503.
pub const C6_TIMEOUT: Duration = Duration::from_millis(150);

/// Reads current prices and FX from Market Data (C6).
pub struct HttpMarketDataClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpMarketDataClient {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(C6_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceDto {
    asset_id: String,
    #[serde(default)]
    price: Option<String>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    observed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    freshness: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FxDto {
    base: String,
    quote: String,
    rate: String,
    #[serde(default)]
    observed_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl MarketDataClient for HttpMarketDataClient {
    async fn prices(&self, asset_ids: &[String]) -> Result<Vec<PriceQuote>, ExternalError> {
        let url = format!("{}/v1/prices", self.base_url);
        let query: Vec<(&str, &str)> = asset_ids
            .iter()
            .map(|asset| ("assetIds", asset.as_str()))
            .collect();
        let response = self
            .client
            .get(url)
            .query(&query)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C6 getPrices: {error}")))?;
        let response = classify(response, "C6 getPrices")?;
        let prices = response
            .json::<Vec<PriceDto>>()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C6 prices body: {error}")))?;
        Ok(prices
            .into_iter()
            .map(|price| PriceQuote {
                asset_id: price.asset_id,
                price: price
                    .price
                    .as_deref()
                    .and_then(|value| value.parse::<Decimal>().ok()),
                currency: price.currency,
                as_of: price.observed_at,
                freshness: price.freshness.unwrap_or_else(|| "unpriced".to_string()),
            })
            .collect())
    }

    async fn fx(&self, base: &str, quote: &str) -> Result<FxQuote, ExternalError> {
        let url = format!("{}/v1/fx", self.base_url);
        let response = self
            .client
            .get(url)
            .query(&[("base", base), ("quote", quote)])
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C6 getFx: {error}")))?;
        let response = classify(response, "C6 getFx")?;
        let fx = response
            .json::<FxDto>()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C6 fx body: {error}")))?;
        let rate = fx.rate.parse::<Decimal>().map_err(|_| {
            ExternalError::Unavailable("C6 fx body carried a non-decimal rate".to_string())
        })?;
        Ok(FxQuote {
            base: fx.base,
            quote: fx.quote,
            rate,
            as_of: fx.observed_at,
        })
    }
}

/// Appends C10 audit records.
pub struct HttpAuditClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpAuditClient {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(AUDIT_CONNECT_TIMEOUT)
            .timeout(AUDIT_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditBody<'a> {
    actor_id: Option<&'a str>,
    actor_kind: &'a str,
    action: &'a str,
    target: &'a str,
    occurred_at: DateTime<Utc>,
    context: BTreeMap<String, String>,
    outcome: &'a str,
    sensitivity: &'a str,
}

#[async_trait]
impl AuditClient for HttpAuditClient {
    async fn append(&self, record: &AuditRecord) -> Result<(), ExternalError> {
        let url = format!("{}/v1/events", self.base_url);
        let body = AuditBody {
            actor_id: record.actor_id.as_deref(),
            actor_kind: &record.actor_kind,
            action: &record.action,
            target: &record.target,
            occurred_at: record.occurred_at,
            context: record.context.clone(),
            outcome: &record.outcome,
            sensitivity: &record.sensitivity,
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C10 append: {error}")))?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        if status == StatusCode::UNPROCESSABLE_ENTITY || status.is_client_error() {
            return Err(ExternalError::Refused(format!(
                "C10 append returned {status}"
            )));
        }
        Err(ExternalError::Unavailable(format!(
            "C10 append returned {status}"
        )))
    }
}

//! The read-only exchange client. Verification makes a signed balances call and a
//! signed permissions check; a key with withdrawal or trading permission is
//! rejected (BR2.1, BR2.5).

use std::time::Duration;

use account_application::ports::{
    ExchangeCredentials, ExchangeProvider, ExternalError, ObservedBalance, VerifiedPermissions,
};
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::str::FromStr;

use crate::http_retry::{is_transient, pause_before_retry};

/// The Binance REST base.
pub const BINANCE_BASE: &str = "https://api.binance.com";
/// The OKX REST base.
pub const OKX_BASE: &str = "https://www.okx.com";

/// Reads read-only venue data over HTTPS.
pub struct HttpExchangeProvider {
    client: reqwest::Client,
    binance_base: String,
    okx_base: String,
}

impl HttpExchangeProvider {
    /// Creates the provider against the production venues.
    #[must_use]
    pub fn new() -> Self {
        Self::with_bases(BINANCE_BASE, OKX_BASE)
    }

    /// Creates the provider against explicit bases, used by tests.
    #[must_use]
    pub fn with_bases(binance_base: impl Into<String>, okx_base: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            binance_base: binance_base.into(),
            okx_base: okx_base.into(),
        }
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<reqwest::Response, ExternalError> {
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            let cloned = request.try_clone().ok_or_else(|| {
                ExternalError::Unavailable(format!("{operation} request cannot be replayed"))
            })?;
            match cloned.send().await {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) if response.status().as_u16() == 429 => {
                    if attempt < 2 {
                        pause_before_retry(attempt).await;
                    } else {
                        return Err(ExternalError::Unavailable(format!(
                            "{operation} was rate-limited"
                        )));
                    }
                }
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "{operation} answered {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    last_error = error.to_string();
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "{operation} is unreachable: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "{operation} did not answer: {last_error}"
        )))
    }

    async fn binance_restrictions(
        &self,
        credentials: &ExchangeCredentials,
    ) -> Result<BinanceRestrictions, ExternalError> {
        let query = format!(
            "timestamp={}&recvWindow=5000",
            Utc::now().timestamp_millis()
        );
        let signature = hmac_sha256_hex(credentials.api_secret.as_bytes(), query.as_bytes());
        let url = format!(
            "{}/sapi/v1/account/apiRestrictions?{query}&signature={signature}",
            self.binance_base.trim_end_matches('/')
        );
        let response = self
            .send(
                self.client
                    .get(url)
                    .header("X-MBX-APIKEY", &credentials.api_key),
                "binance permissions",
            )
            .await?;
        response.json().await.map_err(|error| {
            ExternalError::Unavailable(format!("binance permissions are unreadable: {error}"))
        })
    }

    async fn binance_balances(
        &self,
        credentials: &ExchangeCredentials,
    ) -> Result<Vec<ObservedBalance>, ExternalError> {
        let query = format!(
            "timestamp={}&recvWindow=5000",
            Utc::now().timestamp_millis()
        );
        let signature = hmac_sha256_hex(credentials.api_secret.as_bytes(), query.as_bytes());
        let url = format!(
            "{}/api/v3/account?{query}&signature={signature}",
            self.binance_base.trim_end_matches('/')
        );
        let response = self
            .send(
                self.client
                    .get(url)
                    .header("X-MBX-APIKEY", &credentials.api_key),
                "binance balances",
            )
            .await?;
        let account: BinanceAccount = response.json().await.map_err(|error| {
            ExternalError::Unavailable(format!("binance balances are unreadable: {error}"))
        })?;
        Ok(account
            .balances
            .into_iter()
            .filter_map(|balance| {
                let free = Decimal::from_str(&balance.free).ok()?;
                let locked = Decimal::from_str(&balance.locked).ok()?;
                let quantity = free + locked;
                (!quantity.is_zero()).then_some(ObservedBalance {
                    observed_token: balance.asset,
                    quantity,
                })
            })
            .collect())
    }

    async fn okx_config(
        &self,
        credentials: &ExchangeCredentials,
    ) -> Result<OkxConfig, ExternalError> {
        let path = "/api/v5/account/config";
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let signature = hmac_sha256_base64(
            credentials.api_secret.as_bytes(),
            format!("{timestamp}GET{path}").as_bytes(),
        );
        let url = format!("{}{path}", self.okx_base.trim_end_matches('/'));
        let response = self
            .send(
                self.client
                    .get(url)
                    .header("OK-ACCESS-KEY", &credentials.api_key)
                    .header("OK-ACCESS-SIGN", signature)
                    .header("OK-ACCESS-TIMESTAMP", timestamp)
                    .header(
                        "OK-ACCESS-PASSPHRASE",
                        credentials.passphrase.clone().unwrap_or_default(),
                    ),
                "okx config",
            )
            .await?;
        response.json().await.map_err(|error| {
            ExternalError::Unavailable(format!("okx config is unreadable: {error}"))
        })
    }

    async fn okx_balances(
        &self,
        credentials: &ExchangeCredentials,
    ) -> Result<Vec<ObservedBalance>, ExternalError> {
        let path = "/api/v5/account/balance";
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let signature = hmac_sha256_base64(
            credentials.api_secret.as_bytes(),
            format!("{timestamp}GET{path}").as_bytes(),
        );
        let url = format!("{}{path}", self.okx_base.trim_end_matches('/'));
        let response = self
            .send(
                self.client
                    .get(url)
                    .header("OK-ACCESS-KEY", &credentials.api_key)
                    .header("OK-ACCESS-SIGN", signature)
                    .header("OK-ACCESS-TIMESTAMP", timestamp)
                    .header(
                        "OK-ACCESS-PASSPHRASE",
                        credentials.passphrase.clone().unwrap_or_default(),
                    ),
                "okx balances",
            )
            .await?;
        let balance: OkxBalance = response.json().await.map_err(|error| {
            ExternalError::Unavailable(format!("okx balances are unreadable: {error}"))
        })?;
        Ok(balance
            .data
            .into_iter()
            .flat_map(|entry| entry.details)
            .filter_map(|detail| {
                let quantity = Decimal::from_str(&detail.cash_bal).ok()?;
                (!quantity.is_zero()).then_some(ObservedBalance {
                    observed_token: detail.ccy,
                    quantity,
                })
            })
            .collect())
    }
}

impl Default for HttpExchangeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExchangeProvider for HttpExchangeProvider {
    async fn verify_read_only(
        &self,
        venue: &str,
        credentials: &ExchangeCredentials,
    ) -> Result<VerifiedPermissions, ExternalError> {
        match venue {
            "binance" => {
                let restrictions = self.binance_restrictions(credentials).await?;
                let read_only = restrictions.enable_reading
                    && !restrictions.enable_withdrawals
                    && !restrictions.enable_spot_and_margin_trading
                    && !restrictions.enable_margin
                    && !restrictions.enable_futures;
                Ok(VerifiedPermissions { read_only })
            }
            "okx" => {
                let config = self.okx_config(credentials).await?;
                let read_only = config
                    .data
                    .first()
                    .map(|entry| {
                        let permissions: Vec<&str> = entry.perm.split(',').map(str::trim).collect();
                        !permissions.is_empty()
                            && permissions
                                .iter()
                                .all(|permission| matches!(*permission, "read_only" | "read"))
                    })
                    .unwrap_or(false);
                Ok(VerifiedPermissions { read_only })
            }
            other => Err(ExternalError::Refused(format!(
                "venue '{other}' is not supported"
            ))),
        }
    }

    async fn balances(
        &self,
        venue: &str,
        credentials: &ExchangeCredentials,
    ) -> Result<Vec<ObservedBalance>, ExternalError> {
        match venue {
            "binance" => self.binance_balances(credentials).await,
            "okx" => self.okx_balances(credentials).await,
            other => Err(ExternalError::Refused(format!(
                "venue '{other}' is not supported"
            ))),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct BinanceRestrictions {
    #[serde(default)]
    enable_reading: bool,
    #[serde(default)]
    enable_withdrawals: bool,
    #[serde(default)]
    enable_spot_and_margin_trading: bool,
    #[serde(default)]
    enable_margin: bool,
    #[serde(default)]
    enable_futures: bool,
}

#[derive(Debug, Deserialize)]
struct BinanceAccount {
    balances: Vec<BinanceBalance>,
}

#[derive(Debug, Deserialize)]
struct BinanceBalance {
    asset: String,
    free: String,
    locked: String,
}

#[derive(Debug, Deserialize)]
struct OkxConfig {
    #[serde(default)]
    data: Vec<OkxConfigEntry>,
}

#[derive(Debug, Deserialize)]
struct OkxConfigEntry {
    #[serde(default)]
    perm: String,
}

#[derive(Debug, Deserialize)]
struct OkxBalance {
    #[serde(default)]
    data: Vec<OkxBalanceEntry>,
}

#[derive(Debug, Deserialize)]
struct OkxBalanceEntry {
    #[serde(default)]
    details: Vec<OkxDetail>,
}

#[derive(Debug, Deserialize)]
struct OkxDetail {
    ccy: String,
    #[serde(rename = "cashBal", default)]
    cash_bal: String,
}

/// HMAC-SHA256 over `message`, returned as lowercase hex (Binance's signature).
#[must_use]
pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    hex::encode(hmac_sha256(key, message))
}

/// HMAC-SHA256 over `message`, returned as standard base64 (OKX's signature).
#[must_use]
pub fn hmac_sha256_base64(key: &[u8], message: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(hmac_sha256(key, message))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key);
        block[..32].copy_from_slice(&digest);
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36u8; BLOCK];
    let mut outer_pad = [0x5cu8; BLOCK];
    for index in 0..BLOCK {
        inner_pad[index] ^= block[index];
        outer_pad[index] ^= block[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

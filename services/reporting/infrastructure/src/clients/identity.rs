//! The C3 due-user read.
//!
//! Identity owns user enumeration and timezones; this adapter asks it which
//! users' local boundary for a period has passed and trusts its answer. One
//! attempt inside a short budget: a failure fails the run loudly rather than
//! guessing who is due.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reporting_application::ports::{DueUser, DueUsersClient, ExternalError};
use reporting_domain::{Period, UserId};
use serde::Deserialize;

/// Reads due users from Identity.
pub struct HttpDueUsers {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpDueUsers {
    /// Creates the client with a 300 ms budget.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(150))
            .timeout(Duration::from_millis(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_token: service_token.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DueUserDto {
    user_id: String,
    timezone: String,
}

#[async_trait]
impl DueUsersClient for HttpDueUsers {
    async fn due(&self, period: Period, at: DateTime<Utc>) -> Result<Vec<DueUser>, ExternalError> {
        let url = format!(
            "{}/v1/internal/users/due?period={}&at={}",
            self.base_url.trim_end_matches('/'),
            period.as_str(),
            urlencode(&at.to_rfc3339())
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("the identity due read failed: {error}"))
            })?;
        if !response.status().is_success() {
            return Err(ExternalError::Refused(format!(
                "the identity due read answered {}",
                response.status()
            )));
        }
        let rows = response.json::<Vec<DueUserDto>>().await.map_err(|error| {
            ExternalError::Unavailable(format!("the identity due body was malformed: {error}"))
        })?;
        rows.into_iter()
            .map(|row| {
                let user_id = UserId::parse(&row.user_id).map_err(|_| {
                    ExternalError::Refused(
                        "identity returned a due user id that is not a UUID".to_string(),
                    )
                })?;
                Ok(DueUser {
                    user_id,
                    timezone: row.timezone,
                })
            })
            .collect()
    }
}

/// Percent-encodes the characters an RFC 3339 instant carries inside a query.
#[must_use]
pub fn urlencode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::urlencode;

    #[test]
    fn an_instant_is_percent_encoded() {
        assert_eq!(
            urlencode("2026-11-08T07:00:00+00:00"),
            "2026-11-08T07%3A00%3A00%2B00%3A00"
        );
    }

    #[test]
    fn ordinary_letters_survive() {
        assert_eq!(urlencode("daily"), "daily");
    }
}

//! The geolocation provider adapter behind the `GeoLocator` port (BR1.3, BR1.5).
//!
//! The provider is a third party: its answer is untrusted input and is validated
//! as an ISO 3166-1 alpha-2 country code before it reaches the use case. No raw
//! IP is written to a log (NFR19.1).

use std::time::Duration;

use async_trait::async_trait;
use identity_application::jurisdiction::GeoLocator;
use identity_application::ports::ExternalError;
use identity_domain::CountryCode;

/// Calls the configured provider over HTTPS with the 800 ms read timeout.
pub struct HttpGeoLocator {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl HttpGeoLocator {
    /// Creates the adapter. An empty base URL means the provider is not
    /// configured: every call fails closed with `Unavailable`.
    #[must_use]
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_millis(800))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
        }
    }

    fn country_from_payload(payload: &serde_json::Value) -> Result<CountryCode, ExternalError> {
        let raw = payload
            .get("country_code")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                payload
                    .get("countryCode")
                    .and_then(serde_json::Value::as_str)
            })
            .ok_or_else(|| {
                ExternalError::Unavailable(
                    "the provider answered without a country code".to_string(),
                )
            })?;
        CountryCode::parse(raw).map_err(|error| {
            ExternalError::Unavailable(format!(
                "the provider answered with an invalid country code: {error}"
            ))
        })
    }
}

#[async_trait]
impl GeoLocator for HttpGeoLocator {
    async fn country_for(&self, ip: &str) -> Result<CountryCode, ExternalError> {
        if self.base_url.is_empty() {
            return Err(ExternalError::Unavailable(
                "the geolocation provider is not configured".to_string(),
            ));
        }

        let url = format!("{}/{ip}/json/", self.base_url);
        let mut request = self.client.get(&url);
        if let Some(key) = &self.api_key {
            request = request.header("x-api-key", key);
        }

        let response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                ExternalError::Unavailable("the geolocation provider timed out".to_string())
            } else {
                ExternalError::Unavailable(format!(
                    "the geolocation provider is unreachable: {error}"
                ))
            }
        })?;

        let status = response.status();
        if status.as_u16() == 429 || status.is_server_error() {
            return Err(ExternalError::Unavailable(format!(
                "the geolocation provider answered {status}"
            )));
        }
        if !status.is_success() {
            return Err(ExternalError::Refused(format!(
                "the geolocation provider answered {status}"
            )));
        }

        let payload: serde_json::Value = response.json().await.map_err(|error| {
            ExternalError::Unavailable(format!(
                "the geolocation provider returned an unreadable body: {error}"
            ))
        })?;
        Self::country_from_payload(&payload)
    }
}

#[cfg(test)]
mod tests {
    use super::HttpGeoLocator;

    #[test]
    fn the_provider_payload_accepts_snake_and_camel_case_country_fields() {
        let snake = serde_json::json!({ "country_code": "VN" });
        assert_eq!(
            HttpGeoLocator::country_from_payload(&snake)
                .unwrap()
                .as_str(),
            "VN"
        );

        let camel = serde_json::json!({ "countryCode": "US" });
        assert_eq!(
            HttpGeoLocator::country_from_payload(&camel)
                .unwrap()
                .as_str(),
            "US"
        );
    }

    #[test]
    fn the_provider_payload_refuses_a_missing_or_invalid_country() {
        assert!(HttpGeoLocator::country_from_payload(&serde_json::json!({})).is_err());
        assert!(HttpGeoLocator::country_from_payload(&serde_json::json!({
            "country_code": "vn"
        }))
        .is_err());
    }
}

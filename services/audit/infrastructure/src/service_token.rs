//! The internal service-token registry.
//!
//! A token is issued to exactly one service; the append path derives the emitting
//! service from the token, never from a caller-supplied header. Tokens never live
//! in source or an env var: the file is a Docker secret mounted at
//! `/run/secrets/audit_service_tokens`.

use std::path::Path;

use audit_application::ports::ServiceTokenRegistry;
use serde::Deserialize;

/// A file-backed token registry.
#[derive(Debug, Clone)]
pub struct FileServiceTokens {
    entries: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct TokenFile {
    services: std::collections::BTreeMap<String, String>,
}

impl FileServiceTokens {
    /// Loads the registry from a JSON secret file.
    ///
    /// The file is either `{"asset-registry":"<token>"}` or
    /// `{"services":{"asset-registry":"<token>"}}`.
    ///
    /// # Errors
    /// Returns a message naming the problem when the file is unreadable, is not
    /// JSON, is empty, or maps a service to an empty token.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("{} is unreadable: {error}", path.display()))?;
        Self::from_json(&raw)
    }

    /// Loads the registry from JSON text.
    ///
    /// # Errors
    /// Returns a message naming the problem when the text is not a token map.
    pub fn from_json(raw: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|error| format!("service-token file is not JSON: {error}"))?;
        let map = if value.get("services").is_some() {
            serde_json::from_value::<TokenFile>(value)
                .map_err(|error| format!("service-token file is malformed: {error}"))?
                .services
        } else {
            serde_json::from_value::<std::collections::BTreeMap<String, String>>(value)
                .map_err(|error| format!("service-token file is malformed: {error}"))?
        };
        let mut entries = Vec::with_capacity(map.len());
        for (service, token) in map {
            let service = service.trim().to_string();
            let token = token.trim().to_string();
            if service.is_empty() || token.is_empty() {
                return Err("a service-token entry is empty".to_string());
            }
            if entries
                .iter()
                .any(|(existing, _): &(String, String)| existing == &service)
            {
                return Err(format!("service '{service}' has more than one token"));
            }
            entries.push((service, token));
        }
        if entries.is_empty() {
            return Err("the service-token registry is empty".to_string());
        }
        Ok(Self { entries })
    }

    /// Builds a registry from pairs, used by tests.
    #[must_use]
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self {
            entries: pairs
                .iter()
                .map(|(service, token)| (service.to_string(), token.to_string()))
                .collect(),
        }
    }
}

impl ServiceTokenRegistry for FileServiceTokens {
    fn service_for(&self, token: &str) -> Option<String> {
        let presented = token.trim().as_bytes();
        if presented.is_empty() {
            return None;
        }
        let mut matched = None;
        for (service, expected) in &self.entries {
            if constant_time_eq(presented, expected.as_bytes()) {
                matched = Some(service.clone());
            }
        }
        matched
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in a.iter().zip(b) {
        difference |= left ^ right;
    }
    difference == 0
}

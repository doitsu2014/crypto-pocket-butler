//! BR1.4: event context carries identifiers and metadata only, never a record
//! payload. The allow-list is the one in the security design; the size bound keeps
//! an emitter from turning the trail into a payload store.

use std::collections::BTreeMap;

use crate::error::DomainError;
use crate::secrets::ensure_secret_free;

/// The maximum serialized context size, in bytes.
pub const MAX_CONTEXT_BYTES: usize = 2 * 1024;

/// The maximum number of context entries.
pub const MAX_CONTEXT_KEYS: usize = 5;

/// The keys an event context may carry: correlation id, request id, source
/// service, rule id, and a session or run identifier.
pub const ALLOWED_CONTEXT_KEYS: [&str; 6] = [
    "correlationId",
    "requestId",
    "sourceService",
    "ruleId",
    "sessionId",
    "runId",
];

/// Validated event metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context(BTreeMap<String, String>);

impl Context {
    /// An empty context.
    #[must_use]
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// Validates a raw key-value map.
    ///
    /// # Errors
    /// Returns [`DomainError::ContextKeyNotAllowed`] for an unknown key,
    /// [`DomainError::TooLarge`] when the map exceeds the key or byte bound,
    /// [`DomainError::Invalid`] for a control character, and
    /// [`DomainError::SecretShaped`] for a secret-shaped value.
    pub fn parse(raw: BTreeMap<String, String>) -> Result<Self, DomainError> {
        if raw.len() > MAX_CONTEXT_KEYS {
            return Err(DomainError::TooLarge {
                field: "context",
                limit: "5 keys",
            });
        }

        let mut size = 0usize;
        for (key, value) in &raw {
            if !ALLOWED_CONTEXT_KEYS.contains(&key.as_str()) {
                return Err(DomainError::ContextKeyNotAllowed { key: key.clone() });
            }
            if value.chars().any(char::is_control) {
                return Err(DomainError::Invalid {
                    field: "context",
                    reason: format!("context.{key} must not contain control characters"),
                });
            }
            ensure_secret_free("context", value)?;
            size += key.len() + value.len() + 2;
        }
        if size > MAX_CONTEXT_BYTES {
            return Err(DomainError::TooLarge {
                field: "context",
                limit: "2 KB",
            });
        }
        Ok(Self(raw))
    }

    /// Adds one validated entry. Used by Audit's own paths, which supply internal
    /// identifiers (a session or run id) rather than emitter data.
    ///
    /// # Errors
    /// Returns [`DomainError::ContextKeyNotAllowed`] for an unknown key,
    /// [`DomainError::TooLarge`] when the map would exceed its bounds, and
    /// [`DomainError::SecretShaped`] for a secret-shaped value.
    pub fn insert(&mut self, key: &str, value: &str) -> Result<(), DomainError> {
        if !ALLOWED_CONTEXT_KEYS.contains(&key) {
            return Err(DomainError::ContextKeyNotAllowed {
                key: key.to_string(),
            });
        }
        if !self.0.contains_key(key) && self.0.len() >= MAX_CONTEXT_KEYS {
            return Err(DomainError::TooLarge {
                field: "context",
                limit: "5 keys",
            });
        }
        ensure_secret_free("context", value)?;
        self.0.insert(key.to_string(), value.to_string());
        let size: usize = self
            .0
            .iter()
            .map(|(key, value)| key.len() + value.len() + 2)
            .sum();
        if size > MAX_CONTEXT_BYTES {
            self.0.remove(key);
            return Err(DomainError::TooLarge {
                field: "context",
                limit: "2 KB",
            });
        }
        Ok(())
    }

    /// The entries, in key order.
    #[must_use]
    pub fn as_map(&self) -> &BTreeMap<String, String> {
        &self.0
    }

    /// One value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    /// Whether the context is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// A deterministic representation used by the idempotency payload hash.
    #[must_use]
    pub fn canonical(&self) -> String {
        self.0
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(";")
    }
}

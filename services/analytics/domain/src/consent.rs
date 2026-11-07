//! Consent purpose and the recorded outcome of a per-request check (BR1.1–BR1.3).

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::ConsentCheckId;

/// The purpose a consent decision covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentPurpose {
    /// Error reporting; never gated on analytics consent.
    Essential,
    /// Product analytics; requires a granted analytics consent.
    Analytics,
}

impl ConsentPurpose {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Essential => "essential",
            Self::Analytics => "analytics",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "essential" => Ok(Self::Essential),
            "analytics" => Ok(Self::Analytics),
            _ => Err(DomainError::Invalid {
                field: "consentPurpose",
                reason: "must be essential or analytics".to_string(),
            }),
        }
    }
}

/// The recorded outcome of one consent check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentCheck {
    id: ConsentCheckId,
    purpose: ConsentPurpose,
    granted: bool,
    checked_at: DateTime<Utc>,
    user_ref: Option<String>,
}

impl ConsentCheck {
    /// Records a check outcome. No raw identifier is accepted.
    #[must_use]
    pub fn record(
        id: ConsentCheckId,
        purpose: ConsentPurpose,
        granted: bool,
        checked_at: DateTime<Utc>,
        user_ref: Option<String>,
    ) -> Self {
        Self {
            id,
            purpose,
            granted,
            checked_at,
            user_ref,
        }
    }

    /// Rebuilds a stored row.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when a stored label is unknown.
    pub fn from_stored(
        id: ConsentCheckId,
        purpose: &str,
        granted: bool,
        checked_at: DateTime<Utc>,
        user_ref: Option<String>,
    ) -> Result<Self, DomainError> {
        Ok(Self::record(
            id,
            ConsentPurpose::parse(purpose)?,
            granted,
            checked_at,
            user_ref,
        ))
    }

    /// The stored identifier.
    #[must_use]
    pub fn id(&self) -> ConsentCheckId {
        self.id
    }

    /// The purpose checked.
    #[must_use]
    pub fn purpose(&self) -> ConsentPurpose {
        self.purpose
    }

    /// Whether the purpose was granted.
    #[must_use]
    pub fn granted(&self) -> bool {
        self.granted
    }

    /// When the check ran.
    #[must_use]
    pub fn checked_at(&self) -> DateTime<Utc> {
        self.checked_at
    }

    /// The pseudonymous user reference, when one was available.
    #[must_use]
    pub fn user_ref(&self) -> Option<&str> {
        self.user_ref.as_deref()
    }
}

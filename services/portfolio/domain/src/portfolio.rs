//! The portfolio entity (BR1.2): one named portfolio per user with a base
//! currency.

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{PortfolioId, UserId};

/// The longest accepted portfolio name.
pub const MAX_NAME_CHARS: usize = 60;

/// The name of the portfolio created on demand (BR1.2).
pub const DEFAULT_PORTFOLIO_NAME: &str = "My portfolio";

/// The base currency used when the user has no Identity preference.
pub const DEFAULT_BASE_CURRENCY: &str = "USD";

/// Validates a portfolio name and returns it trimmed.
///
/// # Errors
/// Returns [`DomainError::Required`] for a blank name and
/// [`DomainError::TooLarge`] beyond [`MAX_NAME_CHARS`].
pub fn validate_name(raw: &str) -> Result<String, DomainError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(DomainError::Required { field: "name" });
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(DomainError::TooLarge {
            field: "name",
            limit: "60 characters",
        });
    }
    Ok(name.to_string())
}

/// Normalises and validates an ISO 4217 base currency.
///
/// # Errors
/// Returns [`DomainError::Required`] for a blank value and
/// [`DomainError::Invalid`] when it is not three ASCII letters.
pub fn normalise_currency(raw: &str) -> Result<String, DomainError> {
    let currency = raw.trim().to_ascii_uppercase();
    if currency.is_empty() {
        return Err(DomainError::Required {
            field: "baseCurrency",
        });
    }
    if currency.len() != 3
        || !currency
            .chars()
            .all(|character| character.is_ascii_uppercase())
    {
        return Err(DomainError::Invalid {
            field: "baseCurrency",
            reason: "must be a three-letter ISO 4217 code".to_string(),
        });
    }
    Ok(currency)
}

/// One named portfolio owned by a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Portfolio {
    id: PortfolioId,
    user_id: UserId,
    name: String,
    base_currency: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    anonymised_at: Option<DateTime<Utc>>,
}

impl Portfolio {
    /// Creates a portfolio with a validated name and base currency.
    ///
    /// # Errors
    /// Returns a domain refusal when the name or currency is invalid.
    pub fn new(
        id: PortfolioId,
        user_id: UserId,
        name: &str,
        base_currency: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            user_id,
            name: validate_name(name)?,
            base_currency: normalise_currency(base_currency)?,
            created_at: now,
            updated_at: now,
            anonymised_at: None,
        })
    }

    /// Rehydrates a stored portfolio.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: PortfolioId,
        user_id: UserId,
        name: String,
        base_currency: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        anonymised_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            name,
            base_currency,
            created_at,
            updated_at,
            anonymised_at,
        }
    }

    /// The portfolio identifier.
    #[must_use]
    pub fn id(&self) -> PortfolioId {
        self.id
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The ISO 4217 base currency.
    #[must_use]
    pub fn base_currency(&self) -> &str {
        &self.base_currency
    }

    /// When the portfolio was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the portfolio last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// When the portfolio name was anonymised, if ever.
    #[must_use]
    pub fn anonymised_at(&self) -> Option<DateTime<Utc>> {
        self.anonymised_at
    }
}

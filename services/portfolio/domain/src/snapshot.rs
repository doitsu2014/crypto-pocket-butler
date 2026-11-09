//! Immutable valuation snapshots (BR4.1–BR4.2): one row per portfolio per hour
//! holding the frozen inputs, the computed lines, and their digest. The row is
//! never updated or deleted.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::error::DomainError;
use crate::ids::{PortfolioId, ValuationSnapshotId};

/// The hour bucket used to take at most one scheduled snapshot per portfolio
/// per hour. A retried run therefore writes idempotently.
#[must_use]
pub fn hour_bucket(at: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = at.timestamp();
    let bucket = seconds - seconds.rem_euclid(3600);
    DateTime::from_timestamp(bucket, 0).unwrap_or(at)
}

/// One immutable valuation snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuationSnapshot {
    id: ValuationSnapshotId,
    portfolio_id: PortfolioId,
    as_of: DateTime<Utc>,
    hour_bucket: DateTime<Utc>,
    total_value: Decimal,
    currency: String,
    inputs_digest: String,
    inputs_json: String,
    lines_json: String,
}

impl ValuationSnapshot {
    /// Creates a snapshot. The digest is a 64-character lower-case hex SHA-256
    /// digest over the frozen inputs.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the digest or the frozen JSON is
    /// malformed.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ValuationSnapshotId,
        portfolio_id: PortfolioId,
        as_of: DateTime<Utc>,
        total_value: Decimal,
        currency: &str,
        inputs_digest: String,
        inputs_json: String,
        lines_json: String,
    ) -> Result<Self, DomainError> {
        if inputs_digest.len() != 64
            || !inputs_digest
                .chars()
                .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
        {
            return Err(DomainError::Invalid {
                field: "inputsDigest",
                reason: "must be a 64-character lower-case SHA-256 hex digest".to_string(),
            });
        }
        if inputs_json.trim().is_empty() {
            return Err(DomainError::Required { field: "inputs" });
        }
        if lines_json.trim().is_empty() {
            return Err(DomainError::Required { field: "lines" });
        }
        Ok(Self {
            id,
            portfolio_id,
            as_of,
            hour_bucket: hour_bucket(as_of),
            total_value,
            currency: currency.trim().to_ascii_uppercase(),
            inputs_digest,
            inputs_json,
            lines_json,
        })
    }

    /// Rehydrates a stored snapshot.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: ValuationSnapshotId,
        portfolio_id: PortfolioId,
        as_of: DateTime<Utc>,
        hour_bucket: DateTime<Utc>,
        total_value: Decimal,
        currency: String,
        inputs_digest: String,
        inputs_json: String,
        lines_json: String,
    ) -> Self {
        Self {
            id,
            portfolio_id,
            as_of,
            hour_bucket,
            total_value,
            currency,
            inputs_digest,
            inputs_json,
            lines_json,
        }
    }

    /// The snapshot identifier.
    #[must_use]
    pub fn id(&self) -> ValuationSnapshotId {
        self.id
    }

    /// The portfolio.
    #[must_use]
    pub fn portfolio_id(&self) -> PortfolioId {
        self.portfolio_id
    }

    /// When the valuation was taken.
    #[must_use]
    pub fn as_of(&self) -> DateTime<Utc> {
        self.as_of
    }

    /// The hour bucket that enforces one scheduled snapshot per hour.
    #[must_use]
    pub fn hour_bucket(&self) -> DateTime<Utc> {
        self.hour_bucket
    }

    /// The priced total at `as_of`.
    #[must_use]
    pub fn total_value(&self) -> Decimal {
        self.total_value
    }

    /// The base currency.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The SHA-256 digest over the frozen inputs.
    #[must_use]
    pub fn inputs_digest(&self) -> &str {
        &self.inputs_digest
    }

    /// The frozen inputs as JSON.
    #[must_use]
    pub fn inputs_json(&self) -> &str {
        &self.inputs_json
    }

    /// The computed lines as JSON.
    #[must_use]
    pub fn lines_json(&self) -> &str {
        &self.lines_json
    }
}

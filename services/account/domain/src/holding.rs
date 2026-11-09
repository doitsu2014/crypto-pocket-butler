//! One observed holding in one sync run (BR6.1–BR6.3). Quantities are decimals;
//! binary floating point is never used.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::error::DomainError;
use crate::ids::{AccountId, HoldingSnapshotId, SyncRunId};

/// The most decimal places stored for a quantity.
pub const MAX_QUANTITY_SCALE: u32 = 18;

/// Parses a decimal quantity string.
///
/// # Errors
/// Returns [`DomainError::Invalid`] for an empty value, scientific notation, a
/// negative quantity, or more than [`MAX_QUANTITY_SCALE`] decimal places.
pub fn parse_quantity(raw: &str) -> Result<Decimal, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "quantity" });
    }
    if value.contains('e') || value.contains('E') {
        return Err(DomainError::Invalid {
            field: "quantity",
            reason: "must be a plain decimal string".to_string(),
        });
    }
    let quantity = Decimal::from_str(value).map_err(|_| DomainError::Invalid {
        field: "quantity",
        reason: "must be a decimal number".to_string(),
    })?;
    if quantity.is_sign_negative() {
        return Err(DomainError::Invalid {
            field: "quantity",
            reason: "must not be negative".to_string(),
        });
    }
    if quantity.scale() > MAX_QUANTITY_SCALE {
        return Err(DomainError::Invalid {
            field: "quantity",
            reason: "has more than 18 decimal places".to_string(),
        });
    }
    Ok(quantity)
}

/// The observed holdings for one account at one sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingSnapshot {
    id: HoldingSnapshotId,
    sync_run_id: SyncRunId,
    account_id: AccountId,
    observed_token: String,
    asset_id: Option<String>,
    quantity: Decimal,
    observed_at: DateTime<Utc>,
    spam: bool,
}

impl HoldingSnapshot {
    /// Records one observed token. A token absent from the canonical registry has
    /// no asset id and is labelled spam (BR6.2, BR6.3).
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank observed token.
    pub fn record(
        id: HoldingSnapshotId,
        sync_run_id: SyncRunId,
        account_id: AccountId,
        observed_token: &str,
        asset_id: Option<&str>,
        quantity: Decimal,
        observed_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let observed = observed_token.trim();
        if observed.is_empty() {
            return Err(DomainError::Required {
                field: "observedToken",
            });
        }
        let asset = asset_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        Ok(Self {
            id,
            sync_run_id,
            account_id,
            observed_token: observed.to_string(),
            spam: asset.is_none(),
            asset_id: asset,
            quantity,
            observed_at,
        })
    }

    /// Rehydrates a stored snapshot.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: HoldingSnapshotId,
        sync_run_id: SyncRunId,
        account_id: AccountId,
        observed_token: String,
        asset_id: Option<String>,
        quantity: Decimal,
        observed_at: DateTime<Utc>,
        spam: bool,
    ) -> Self {
        Self {
            id,
            sync_run_id,
            account_id,
            observed_token,
            asset_id,
            quantity,
            observed_at,
            spam,
        }
    }

    /// The snapshot identifier.
    #[must_use]
    pub fn id(&self) -> HoldingSnapshotId {
        self.id
    }

    /// The run that produced the snapshot.
    #[must_use]
    pub fn sync_run_id(&self) -> SyncRunId {
        self.sync_run_id
    }

    /// The owning account.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// The token as the venue or chain reported it.
    #[must_use]
    pub fn observed_token(&self) -> &str {
        &self.observed_token
    }

    /// The canonical asset id, absent for a spam holding.
    #[must_use]
    pub fn asset_id(&self) -> Option<&str> {
        self.asset_id.as_deref()
    }

    /// The decimal quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// When the holding was observed.
    #[must_use]
    pub fn observed_at(&self) -> DateTime<Utc> {
        self.observed_at
    }

    /// Whether the asset is absent from the canonical registry.
    #[must_use]
    pub fn spam(&self) -> bool {
        self.spam
    }
}

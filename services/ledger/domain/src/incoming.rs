//! An imported row as Account's C5 returns it (BR1.4).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::transaction::{Kind, Source};

/// One C5 row. A row with a line error is reported and never stored; a row
/// missing a required field is treated the same way.
#[derive(Debug, Clone, PartialEq)]
pub struct IncomingTransaction {
    /// The connected account.
    pub account_id: Option<Uuid>,
    /// The asset.
    pub asset_id: String,
    /// The parsed kind, absent when the row is malformed.
    pub kind: Option<Kind>,
    /// The parsed quantity.
    pub quantity: Option<Decimal>,
    /// The parsed unit price.
    pub price: Option<Decimal>,
    /// The ISO currency the price is quoted in, when the source reports one.
    pub currency: Option<String>,
    /// The execution time.
    pub executed_at: Option<DateTime<Utc>>,
    /// The source system's row identifier.
    pub external_ref: Option<String>,
    /// Whether the row came from a venue or a CSV.
    pub source: Source,
    /// The line-level error Account reported, if any.
    pub line_error: Option<String>,
}

impl IncomingTransaction {
    /// The reason the row cannot be stored, if any (BR1.4).
    #[must_use]
    pub fn refusal(&self) -> Option<String> {
        if let Some(error) = &self.line_error {
            return Some(error.clone());
        }
        if self.asset_id.trim().is_empty() {
            return Some("assetId is missing".to_string());
        }
        if self.kind.is_none() {
            return Some("kind is missing or unknown".to_string());
        }
        if self.quantity.is_none() {
            return Some("quantity is missing or not a decimal".to_string());
        }
        if self.executed_at.is_none() {
            return Some("executedAt is missing or not a timestamp".to_string());
        }
        None
    }
}

//! Corrections: append-only restatements of one field (BR1.1, BR2.3, BR4.1).

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;

use crate::constants::FUTURE_SKEW_SECONDS;
use crate::error::DomainError;
use crate::ids::{CorrectionId, TransactionId};
use crate::transaction::{canonical_decimal, Kind, Transaction};

/// The single field a correction restates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorrectionField {
    /// The moved quantity.
    Quantity,
    /// The unit price.
    Price,
    /// The movement kind.
    Kind,
    /// The execution timestamp.
    ExecutedAt,
    /// The asset.
    AssetId,
}

impl CorrectionField {
    /// The stored spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quantity => "quantity",
            Self::Price => "price",
            Self::Kind => "kind",
            Self::ExecutedAt => "executedAt",
            Self::AssetId => "assetId",
        }
    }

    /// Parses the wire spelling.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown field.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "quantity" => Ok(Self::Quantity),
            "price" => Ok(Self::Price),
            "kind" => Ok(Self::Kind),
            "executedAt" => Ok(Self::ExecutedAt),
            "assetId" => Ok(Self::AssetId),
            _ => Err(DomainError::Invalid {
                field: "field",
                reason: "must be one of quantity, price, kind, executedAt, assetId".to_string(),
            }),
        }
    }

    /// Every field a correction may restate.
    #[must_use]
    pub fn all() -> [Self; 5] {
        [
            Self::Quantity,
            Self::Price,
            Self::Kind,
            Self::ExecutedAt,
            Self::AssetId,
        ]
    }
}

/// A parsed correction value.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// A quantity.
    Quantity(Decimal),
    /// A price.
    Price(Decimal),
    /// A kind.
    Kind(Kind),
    /// An execution timestamp.
    ExecutedAt(DateTime<Utc>),
    /// An asset identifier.
    AssetId(String),
}

impl FieldValue {
    /// The value's stored text.
    #[must_use]
    pub fn as_text(&self) -> String {
        match self {
            Self::Quantity(value) | Self::Price(value) => canonical_decimal(*value),
            Self::Kind(kind) => kind.as_str().to_string(),
            Self::ExecutedAt(value) => value.to_rfc3339(),
            Self::AssetId(value) => value.clone(),
        }
    }
}

/// One append-only correction row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    id: CorrectionId,
    transaction_id: TransactionId,
    field: CorrectionField,
    old_value: String,
    new_value: String,
    reason: String,
    actor_id: String,
    corrected_at: DateTime<Utc>,
}

impl Correction {
    /// Records a correction row. Validity of the value is checked by
    /// [`parse_field_value`] at the boundary and by the use case before this
    /// call; this constructor checks only presence.
    ///
    /// # Errors
    /// Returns [`DomainError`] when the reason or new value is blank.
    pub fn record(
        id: CorrectionId,
        transaction_id: TransactionId,
        field: CorrectionField,
        old_value: impl Into<String>,
        new_value: impl Into<String>,
        reason: impl Into<String>,
        actor_id: impl Into<String>,
        corrected_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(DomainError::Required { field: "reason" });
        }
        let new_value = new_value.into();
        if new_value.trim().is_empty() {
            return Err(DomainError::Required { field: "newValue" });
        }
        let actor_id = actor_id.into();
        if actor_id.trim().is_empty() {
            return Err(DomainError::Required { field: "actorId" });
        }
        Ok(Self {
            id,
            transaction_id,
            field,
            old_value: old_value.into(),
            new_value: new_value.trim().to_string(),
            reason: reason.trim().to_string(),
            actor_id,
            corrected_at,
        })
    }

    /// Rehydrates a stored correction.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: CorrectionId,
        transaction_id: TransactionId,
        field: CorrectionField,
        old_value: String,
        new_value: String,
        reason: String,
        actor_id: String,
        corrected_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            transaction_id,
            field,
            old_value,
            new_value,
            reason,
            actor_id,
            corrected_at,
        }
    }

    /// The correction identifier.
    #[must_use]
    pub fn id(&self) -> CorrectionId {
        self.id
    }

    /// The corrected transaction.
    #[must_use]
    pub fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }

    /// The restated field.
    #[must_use]
    pub fn field(&self) -> CorrectionField {
        self.field
    }

    /// The value that was replaced.
    #[must_use]
    pub fn old_value(&self) -> &str {
        &self.old_value
    }

    /// The replacement value.
    #[must_use]
    pub fn new_value(&self) -> &str {
        &self.new_value
    }

    /// Why the correction was made; never a credential.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The acting user.
    #[must_use]
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// When it was recorded.
    #[must_use]
    pub fn corrected_at(&self) -> DateTime<Utc> {
        self.corrected_at
    }
}

/// Validates a correction value for its field (BR4.1).
///
/// # Errors
/// Returns [`DomainError`] naming the field when the value is blank, not a
/// decimal, not positive, negative, an unknown kind, or a future timestamp.
pub fn parse_field_value(
    field: CorrectionField,
    raw: &str,
    now: DateTime<Utc>,
) -> Result<FieldValue, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "newValue" });
    }
    match field {
        CorrectionField::Quantity => {
            let quantity = parse_decimal("newValue", value)?;
            if quantity <= Decimal::ZERO {
                return Err(DomainError::Invalid {
                    field: "newValue",
                    reason: "quantity must be greater than zero".to_string(),
                });
            }
            Ok(FieldValue::Quantity(quantity))
        }
        CorrectionField::Price => {
            let price = parse_decimal("newValue", value)?;
            if price < Decimal::ZERO {
                return Err(DomainError::Invalid {
                    field: "newValue",
                    reason: "price must be zero or greater".to_string(),
                });
            }
            Ok(FieldValue::Price(price))
        }
        CorrectionField::Kind => Ok(FieldValue::Kind(Kind::parse(value)?)),
        CorrectionField::ExecutedAt => {
            let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| DomainError::Invalid {
                field: "newValue",
                reason: "executedAt must be an RFC 3339 timestamp".to_string(),
            })?;
            let executed_at = parsed.with_timezone(&Utc);
            if executed_at > now + Duration::seconds(FUTURE_SKEW_SECONDS) {
                return Err(DomainError::Invalid {
                    field: "newValue",
                    reason: "executedAt must not be in the future".to_string(),
                });
            }
            Ok(FieldValue::ExecutedAt(executed_at))
        }
        CorrectionField::AssetId => Ok(FieldValue::AssetId(value.to_string())),
    }
}

fn parse_decimal(field: &'static str, raw: &str) -> Result<Decimal, DomainError> {
    raw.parse::<Decimal>().map_err(|_| DomainError::Invalid {
        field,
        reason: "must be a decimal string".to_string(),
    })
}

/// The value in force for `field` after applying every earlier correction to
/// that field (BR4.1). This is the `oldValue` a new correction records.
#[must_use]
pub fn effective_value(
    transaction: &Transaction,
    corrections: &[Correction],
    field: CorrectionField,
) -> String {
    let mut value = base_value(transaction, field);
    let mut ordered: Vec<&Correction> = corrections
        .iter()
        .filter(|correction| correction.field() == field)
        .collect();
    ordered.sort_by_key(|correction| (correction.corrected_at(), correction.id().as_uuid()));
    for correction in ordered {
        value = correction.new_value().to_string();
    }
    value
}

fn base_value(transaction: &Transaction, field: CorrectionField) -> String {
    match field {
        CorrectionField::Quantity => canonical_decimal(transaction.quantity()),
        CorrectionField::Price => transaction
            .price()
            .map_or_else(|| "none".to_string(), canonical_decimal),
        CorrectionField::Kind => transaction.kind().as_str().to_string(),
        CorrectionField::ExecutedAt => transaction.executed_at().to_rfc3339(),
        CorrectionField::AssetId => transaction.asset_id().to_string(),
    }
}

/// The transaction's field values after every correction, in the order the
/// corrections were recorded (BR2.3).
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveTransaction {
    /// The transaction identifier.
    pub id: TransactionId,
    /// The corrected asset.
    pub asset_id: String,
    /// The corrected kind.
    pub kind: Kind,
    /// The corrected quantity.
    pub quantity: Decimal,
    /// The corrected price.
    pub price: Option<Decimal>,
    /// The ISO currency the price is quoted in (never corrected).
    pub currency: String,
    /// The corrected execution time.
    pub executed_at: DateTime<Utc>,
    /// The source system, never corrected.
    pub source: crate::transaction::Source,
}

/// Applies a transaction's corrections to produce the view the lot engine
/// replays.
///
/// # Errors
/// Returns [`DomainError`] when a stored correction's value is malformed, so a
/// bad row is surfaced rather than silently applied.
pub fn effective_transaction(
    transaction: &Transaction,
    corrections: &[Correction],
) -> Result<EffectiveTransaction, DomainError> {
    let mut asset_id = transaction.asset_id().to_string();
    let mut kind = transaction.kind();
    let mut quantity = transaction.quantity();
    let mut price = transaction.price();
    let mut executed_at = transaction.executed_at();

    let mut ordered: Vec<&Correction> = corrections.iter().collect();
    ordered.sort_by_key(|correction| (correction.corrected_at(), correction.id().as_uuid()));
    for correction in ordered {
        let now = correction.corrected_at();
        match parse_field_value(correction.field(), correction.new_value(), now)? {
            FieldValue::Quantity(value) => quantity = value,
            FieldValue::Price(value) => price = Some(value),
            FieldValue::Kind(value) => kind = value,
            FieldValue::ExecutedAt(value) => executed_at = value,
            FieldValue::AssetId(value) => asset_id = value,
        }
    }
    Ok(EffectiveTransaction {
        id: transaction.id(),
        asset_id,
        kind,
        quantity,
        price,
        currency: transaction.currency().to_string(),
        executed_at,
        source: transaction.source(),
    })
}

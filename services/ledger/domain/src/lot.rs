//! FIFO cost-basis lots (BR2.1, BR2.4, BR2.5).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::error::DomainError;
use crate::ids::{CostBasisLotId, LedgerSubjectId, TransactionId};

/// One acquisition lot. Its remaining quantity never goes negative (NFR1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct CostBasisLot {
    id: CostBasisLotId,
    subject_id: LedgerSubjectId,
    asset_id: String,
    acquired_at: DateTime<Utc>,
    quantity: Decimal,
    remaining: Decimal,
    unit_cost: Decimal,
    currency: String,
    source_transaction_id: TransactionId,
}

impl CostBasisLot {
    /// Creates a lot from an acquisition's total cost. A fee already attached
    /// to the trade is capitalised into `total_cost`.
    ///
    /// # Errors
    /// Returns [`DomainError`] when the quantity is not positive.
    pub fn acquire(
        id: CostBasisLotId,
        subject_id: LedgerSubjectId,
        asset_id: impl Into<String>,
        acquired_at: DateTime<Utc>,
        quantity: Decimal,
        total_cost: Decimal,
        currency: impl Into<String>,
        source_transaction_id: TransactionId,
    ) -> Result<Self, DomainError> {
        if quantity <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "quantity",
                reason: "a lot's quantity must be greater than zero".to_string(),
            });
        }
        Ok(Self {
            id,
            subject_id,
            asset_id: asset_id.into(),
            acquired_at,
            quantity,
            remaining: quantity,
            unit_cost: total_cost / quantity,
            currency: crate::fx::normalise_currency(&currency.into()),
            source_transaction_id,
        })
    }

    /// Rehydrates a stored lot.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: CostBasisLotId,
        subject_id: LedgerSubjectId,
        asset_id: String,
        acquired_at: DateTime<Utc>,
        quantity: Decimal,
        remaining: Decimal,
        unit_cost: Decimal,
        currency: String,
        source_transaction_id: TransactionId,
    ) -> Self {
        Self {
            id,
            subject_id,
            asset_id,
            acquired_at,
            quantity,
            remaining,
            unit_cost,
            currency,
            source_transaction_id,
        }
    }

    /// The identifier.
    #[must_use]
    pub fn id(&self) -> CostBasisLotId {
        self.id
    }

    /// The owning subject.
    #[must_use]
    pub fn subject_id(&self) -> LedgerSubjectId {
        self.subject_id
    }

    /// The asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// When the asset was acquired.
    #[must_use]
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }

    /// The original quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// The un-consumed quantity.
    #[must_use]
    pub fn remaining(&self) -> Decimal {
        self.remaining
    }

    /// The cost per unit, in [`currency`](Self::currency).
    #[must_use]
    pub fn unit_cost(&self) -> Decimal {
        self.unit_cost
    }

    /// The ISO currency the unit cost is quoted in.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The transaction that created the lot.
    #[must_use]
    pub fn source_transaction_id(&self) -> TransactionId {
        self.source_transaction_id
    }

    /// The cost still held.
    #[must_use]
    pub fn remaining_cost(&self) -> Decimal {
        self.remaining * self.unit_cost
    }

    /// Whether the lot still holds quantity.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.remaining > Decimal::ZERO
    }

    /// Consumes up to `quantity` from this lot and returns the consumed cost.
    #[must_use]
    pub fn consume(&mut self, quantity: Decimal) -> Decimal {
        let taken = quantity.min(self.remaining).max(Decimal::ZERO);
        let cost = taken * self.unit_cost;
        self.remaining -= taken;
        cost
    }
}

/// Consumes the oldest lots first and returns the consumed quantity and cost.
/// The returned quantity is less than the request when the lots cannot cover
/// it; the caller records the shortfall as a coverage gap (NFR1.7).
#[must_use]
pub fn consume_fifo(lots: &mut [CostBasisLot], quantity: Decimal) -> (Decimal, Decimal) {
    let mut needed = quantity.max(Decimal::ZERO);
    let mut consumed_quantity = Decimal::ZERO;
    let mut consumed_cost = Decimal::ZERO;
    if needed <= Decimal::ZERO {
        return (consumed_quantity, consumed_cost);
    }
    lots.sort_by_key(|lot| (lot.acquired_at(), lot.id().as_uuid()));
    for lot in lots.iter_mut() {
        if needed <= Decimal::ZERO {
            break;
        }
        let before = lot.remaining();
        let cost = lot.consume(needed);
        let taken = before - lot.remaining();
        consumed_quantity += taken;
        consumed_cost += cost;
        needed -= taken;
    }
    (consumed_quantity, consumed_cost)
}

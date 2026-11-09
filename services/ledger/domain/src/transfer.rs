//! Transfer pairing: a transfer never becomes a taxable disposal (BR2.4, BR6.1).

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;

use crate::constants::{TRANSFER_QUANTITY_TOLERANCE_BPS, TRANSFER_WINDOW_HOURS};
use crate::error::DomainError;
use crate::ids::{LedgerSubjectId, TransactionId, TransferPairId};
use crate::transaction::{Kind, Transaction};

/// The state of a transfer leg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferStatus {
    /// Waiting for a counter-entry.
    Pending,
    /// Paired with its counter-entry.
    Matched,
    /// The user declined the pairing.
    Rejected,
}

impl TransferStatus {
    /// The stored spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Matched => "matched",
            Self::Rejected => "rejected",
        }
    }

    /// Parses the stored spelling.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "pending" => Ok(Self::Pending),
            "matched" => Ok(Self::Matched),
            "rejected" => Ok(Self::Rejected),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be one of pending, matched, rejected".to_string(),
            }),
        }
    }
}

/// A link between a transfer_out and its matching transfer_in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferPair {
    id: TransferPairId,
    subject_id: LedgerSubjectId,
    out_transaction_id: Option<TransactionId>,
    in_transaction_id: Option<TransactionId>,
    asset_id: String,
    status: TransferStatus,
    matched_at: Option<DateTime<Utc>>,
}

impl TransferPair {
    /// Records a pending pair for an outgoing leg.
    #[must_use]
    pub fn pending(
        id: TransferPairId,
        subject_id: LedgerSubjectId,
        out_transaction_id: TransactionId,
        asset_id: impl Into<String>,
    ) -> Self {
        Self {
            id,
            subject_id,
            out_transaction_id: Some(out_transaction_id),
            in_transaction_id: None,
            asset_id: asset_id.into(),
            status: TransferStatus::Pending,
            matched_at: None,
        }
    }

    /// Records a pending pair for an incoming leg that has no outbound leg yet.
    #[must_use]
    pub fn pending_in(
        id: TransferPairId,
        subject_id: LedgerSubjectId,
        in_transaction_id: TransactionId,
        asset_id: impl Into<String>,
    ) -> Self {
        Self {
            id,
            subject_id,
            out_transaction_id: None,
            in_transaction_id: Some(in_transaction_id),
            asset_id: asset_id.into(),
            status: TransferStatus::Pending,
            matched_at: None,
        }
    }

    /// Rehydrates a stored pair.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: TransferPairId,
        subject_id: LedgerSubjectId,
        out_transaction_id: Option<TransactionId>,
        in_transaction_id: Option<TransactionId>,
        asset_id: String,
        status: TransferStatus,
        matched_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            subject_id,
            out_transaction_id,
            in_transaction_id,
            asset_id,
            status,
            matched_at,
        }
    }

    /// The identifier.
    #[must_use]
    pub fn id(&self) -> TransferPairId {
        self.id
    }

    /// The owning subject.
    #[must_use]
    pub fn subject_id(&self) -> LedgerSubjectId {
        self.subject_id
    }

    /// The outgoing leg, when one is known.
    #[must_use]
    pub fn out_transaction_id(&self) -> Option<TransactionId> {
        self.out_transaction_id
    }

    /// The incoming leg, once matched.
    #[must_use]
    pub fn in_transaction_id(&self) -> Option<TransactionId> {
        self.in_transaction_id
    }

    /// The asset both legs carry.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The current status.
    #[must_use]
    pub fn status(&self) -> TransferStatus {
        self.status
    }

    /// When the pair matched.
    #[must_use]
    pub fn matched_at(&self) -> Option<DateTime<Utc>> {
        self.matched_at
    }

    /// Whether the pair is matched, so neither leg is a disposal or an
    /// independent priced acquisition (BR2.4, BR6.1).
    #[must_use]
    pub fn is_matched(&self) -> bool {
        self.status == TransferStatus::Matched
    }

    /// Marks the pair matched with its outgoing and incoming legs.
    pub fn mark_matched(
        &mut self,
        out_transaction_id: TransactionId,
        in_transaction_id: TransactionId,
        matched_at: DateTime<Utc>,
    ) {
        self.out_transaction_id = Some(out_transaction_id);
        self.in_transaction_id = Some(in_transaction_id);
        self.status = TransferStatus::Matched;
        self.matched_at = Some(matched_at);
    }

    /// Marks the pair rejected after an explicit user refusal (BR6.1).
    pub fn reject(&mut self) {
        self.status = TransferStatus::Rejected;
    }
}

/// Finds the nearest unmatched counter-entry of the same asset whose quantity
/// is within one percent and whose time is within twenty-four hours (BR6.1).
///
/// `out` is the leg being paired; `candidates` are the transfer transactions of
/// the opposite kind. An explicit user rejection is applied by the caller and
/// removes the leg from this search.
#[must_use]
pub fn find_match(out: &Transaction, candidates: &[Transaction]) -> Option<TransactionId> {
    let opposite = match out.kind() {
        Kind::TransferOut => Kind::TransferIn,
        Kind::TransferIn => Kind::TransferOut,
        _ => return None,
    };
    let tolerance = Decimal::new(TRANSFER_QUANTITY_TOLERANCE_BPS, 4);
    let window = Duration::hours(TRANSFER_WINDOW_HOURS);
    candidates
        .iter()
        .filter(|candidate| candidate.kind() == opposite)
        .filter(|candidate| candidate.asset_id() == out.asset_id())
        .filter(|candidate| quantity_within(candidate.quantity(), out.quantity(), tolerance))
        .filter(|candidate| {
            let gap = candidate.executed_at() - out.executed_at();
            gap.abs() <= window
        })
        .min_by_key(|candidate| {
            let gap = (candidate.executed_at() - out.executed_at())
                .num_seconds()
                .abs();
            (gap, candidate.executed_at(), candidate.id().as_uuid())
        })
        .map(Transaction::id)
}

fn quantity_within(left: Decimal, right: Decimal, tolerance: Decimal) -> bool {
    let largest = left.max(right);
    if largest <= Decimal::ZERO {
        return left == right;
    }
    (left - right).abs() <= largest * tolerance
}

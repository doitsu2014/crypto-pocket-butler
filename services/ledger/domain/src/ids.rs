//! Typed UUID identifiers, so a transaction id cannot stand in for a lot id.

use std::fmt;

use uuid::Uuid;

use crate::error::DomainError;

macro_rules! uuid_id {
    ($name:ident, $doc:literal, $entity:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            /// Wraps an existing UUID.
            #[must_use]
            pub fn new(id: Uuid) -> Self {
                Self(id)
            }

            /// Generates a fresh random identifier.
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }

            /// Parses a UUID string.
            ///
            /// # Errors
            /// Returns [`DomainError::InvalidId`] when the value is not a UUID.
            pub fn parse(raw: &str) -> Result<Self, DomainError> {
                Uuid::parse_str(raw.trim())
                    .map(Self)
                    .map_err(|_| DomainError::InvalidId { entity: $entity })
            }

            /// The underlying UUID.
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = DomainError;

            fn from_str(raw: &str) -> Result<Self, Self::Err> {
                Self::parse(raw)
            }
        }
    };
}

uuid_id!(
    TransactionId,
    "The identifier of one append-only ledger transaction.",
    "transaction"
);
uuid_id!(
    CorrectionId,
    "The identifier of one append-only correction.",
    "correction"
);
uuid_id!(
    TransferPairId,
    "The identifier of one transfer pairing record.",
    "transfer pair"
);
uuid_id!(
    CostBasisLotId,
    "The identifier of one FIFO cost-basis lot.",
    "cost-basis lot"
);
uuid_id!(
    PnlSnapshotId,
    "The identifier of one computed P&L snapshot.",
    "P&L snapshot"
);
uuid_id!(
    LedgerSubjectId,
    "The identifier of one mutable ledger subject (the anonymisation handle).",
    "ledger subject"
);

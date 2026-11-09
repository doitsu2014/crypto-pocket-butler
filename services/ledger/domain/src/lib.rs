//! Ledger domain: append-only transactions, corrections, FIFO cost basis,
//! transfer pairing, P&L and coverage.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod constants;
pub mod correction;
pub mod engine;
pub mod error;
pub mod fx;
pub mod ids;
pub mod incoming;
pub mod lot;
pub mod pnl;
pub mod snapshot;
pub mod subject;
pub mod transaction;
pub mod transfer;

pub use constants::{
    AUDIT_RETRY_SECONDS, FUTURE_SKEW_SECONDS, LIST_PAGE_DEFAULT, LIST_PAGE_MAX,
    RECONCILE_BATCH_TRANSACTIONS, RECONCILE_LEASE_SECONDS, RETENTION_YEARS,
    TRANSFER_QUANTITY_TOLERANCE_BPS, TRANSFER_WINDOW_HOURS,
};
pub use correction::{
    effective_transaction, effective_value, parse_field_value, Correction, CorrectionField,
    EffectiveTransaction, FieldValue,
};
pub use engine::{replay, ReplayOutcome};
pub use error::DomainError;
pub use fx::{normalise_currency, FxRates};
pub use ids::{
    CorrectionId, CostBasisLotId, LedgerSubjectId, PnlSnapshotId, TransactionId, TransferPairId,
};
pub use incoming::IncomingTransaction;
pub use lot::{consume_fifo, CostBasisLot};
pub use pnl::{
    compute_pnl, AssetPnl, CoverageLabel, HeldAsset, PnlReport, PriceFreshness, PriceMark,
};
pub use snapshot::PnlSnapshot;
pub use subject::LedgerSubject;
pub use transaction::{
    canonical_decimal, canonical_key, Kind, NewTransaction, PriceSource, Source, Transaction,
};
pub use transfer::{find_match, TransferPair, TransferStatus};

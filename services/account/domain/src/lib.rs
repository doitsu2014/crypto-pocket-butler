//! Account domain: read-only holding sources, exchange credentials, wallet
//! connections, sync runs with a lease, per-sync snapshots, and CSV imports.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod account;
pub mod audit;
pub mod credential;
pub mod error;
pub mod holding;
pub mod ids;
pub mod import;
pub mod secrets;
pub mod service;
pub mod sync;
pub mod wallet;

pub use account::{
    normalise_label, validate_label, Account, AccountKind, AccountStatus, Venue, MAX_LABEL_CHARS,
};
pub use audit::{AuditOutcome, AuditRecord};
pub use credential::{Credential, CredentialStatus};
pub use error::DomainError;
pub use holding::{parse_quantity, HoldingSnapshot};
pub use ids::{
    AccountId, CredentialId, HoldingSnapshotId, ImportBatchId, ImportRowErrorId,
    ImportedTransactionId, SyncRunId, UserId, WalletConnectionId,
};
pub use import::{
    dedupe_key_for, parse_csv_line, ExportedAccount, ImportBatch, ImportBatchStatus,
    ImportRowError, ImportSource, ImportSummary, ImportedTransaction, ParsedTransaction,
    TransactionKind, CSV_MAX_BYTES,
};
pub use secrets::{ensure_not_secret, looks_like_secret};
pub use service::{service_allowed, Operation};
pub use sync::{
    ErrorCategory, SyncRun, SyncRunStatus, LEASE_SECONDS, MAX_ERROR_MESSAGE_CHARS,
    RENEW_INTERVAL_SECONDS, SWEEP_INTERVAL_SECONDS,
};
pub use wallet::{validate_address, WalletConnection};

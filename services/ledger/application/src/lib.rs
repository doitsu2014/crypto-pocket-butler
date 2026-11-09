//! Ledger application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `ledger-domain` only.

pub mod common;
pub mod corrections;
pub mod hashing;
pub mod ingest;
pub mod ports;
pub mod privacy;
pub mod reads;
pub mod rebuild;
pub mod reconcile;
pub mod trades;
pub mod trail;

pub use common::{AppError, FieldError};
pub use corrections::{CorrectionResult, CorrectionsService};
pub use hashing::sha256_hex;
pub use ingest::{IngestReport, IngestService, RowRefusal};
pub use ports::{
    AccountClient, AnonymisationRepository, AnonymiserGateway, AssetResolver, AuditClient,
    AuditRecord, AuthError, Clock, CorrectionRepository, ExternalError, FxQuote, Holding,
    IdGenerator, LotRepository, MarketDataClient, Pause, PriceQuote, Principal, ReadinessProbe,
    ReconcileLease, RepositoryError, Role, ServiceCaller, ServiceTokenRegistry, SnapshotRepository,
    SubjectRepository, TokenValidator, TransactionCursor, TransactionRepository,
    TransferPairRepository,
};
pub use privacy::{AnonymiseResult, PrivacyExport, PrivacyService};
pub use reads::{decode_cursor, encode_cursor, PnlService, ReadService, TransactionPage};
pub use rebuild::{build_effective, Rebuilder};
pub use reconcile::{ReconcileReport, ReconcileService, ReconcileStatus};
pub use trades::{RecordedTrade, TradeCommand, TradesService};
pub use trail::Trail;

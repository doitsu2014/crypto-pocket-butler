//! Market Data application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `market-data-domain` only.

pub mod common;
pub mod fx_service;
pub mod ports;
pub mod quotes;
pub mod refresh;
pub mod retention;

pub use common::{AppError, FieldError, ServiceCaller};
pub use fx_service::FxService;
pub use ports::{
    AssetRegistry, AuditClient, CanonicalAsset, ClaimOutcome, Clock, ExternalError, FxRepository,
    NoopMetrics, ObservationRepository, PriceProvider, PriceSource, ProviderError, ProviderQuote,
    ProviderSecrets, RateLimiter, ReadinessProbe, RefreshRunRepository, RegisteredService,
    RepositoryError, ServiceMetrics, ServiceTokenRegistry, Sleeper, TierRepository,
};
pub use quotes::QuoteService;
pub use refresh::{RefreshService, RunOutcome};
pub use retention::{RetentionReport, RetentionService};

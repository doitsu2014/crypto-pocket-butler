//! Analytics infrastructure adapters.

pub mod auth;
pub mod clients;
pub mod clock;
pub mod db;
pub mod migrations;
pub mod pause;
pub mod readiness;
pub mod repositories;
pub mod service_token;
pub mod vault;

pub use auth::{HttpJwksSource, JwkKey, JwksSource, JwksValidator};
pub use clients::{HttpAuditClient, HttpConsentChecker};
pub use clock::SystemClock;
pub use db::connect;
pub use pause::TokioPause;
pub use readiness::AnalyticsReadinessProbe;
pub use repositories::{
    PgAggregateRepository, PgConsentRepository, PgEventRepository, PgFlagRepository,
};
pub use service_token::PgServiceTokenRegistry;
pub use vault::{decode_salt, PgSaltProvider};

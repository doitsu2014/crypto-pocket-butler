//! Asset Registry infrastructure adapters: sqlx repositories, JWKS validation,
//! and the local audit sink.

pub mod audit;
pub mod auth;
pub mod clock;
pub mod db;
pub mod migrations;
pub mod price;
pub mod readiness;
pub mod repositories;

pub use audit::TracingAuditSink;
pub use auth::{HttpJwksSource, JwkKey, JwksSource, JwksValidator};
pub use clock::SystemClock;
pub use db::connect;
pub use price::NoPriceProvider;
pub use readiness::PgReadinessProbe;

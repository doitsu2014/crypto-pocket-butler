//! Reporting infrastructure adapters: sqlx repositories, the C3/C7/C8/C9/C10
//! clients, JWT validation, and the service-token registry.

pub mod audit_client;
pub mod auth;
pub mod clients;
pub mod clock;
pub mod db;
pub mod http_retry;
pub mod ids;
pub mod migrations;
pub mod notification_client;
pub mod readiness;
pub mod repositories;
pub mod service_token;

pub use audit_client::HttpAuditClient;
pub use auth::{HttpJwksSource, JwkKey, JwksSource, JwksValidator};
pub use clock::SystemClock;
pub use db::connect;
pub use ids::UuidGenerator;
pub use notification_client::HttpNotification;
pub use readiness::PgReadinessProbe;
pub use service_token::{hash_token, PgServiceTokenRegistry};

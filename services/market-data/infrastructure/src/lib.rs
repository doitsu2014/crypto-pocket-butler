//! Market Data infrastructure adapters: sqlx repositories, the C4 client, the
//! provider client, Vault, Audit, and JWT-free service-token resolution.

pub mod audit_client;
pub mod clock;
pub mod db;
pub mod migrations;
pub mod provider;
pub mod readiness;
pub mod registry_client;
pub mod repositories;
pub mod service_token;
pub mod sleeper;
pub mod vault;

pub use audit_client::HttpAuditClient;
pub use clock::SystemClock;
pub use db::connect;
pub use provider::HttpPriceProvider;
pub use readiness::PgReadinessProbe;
pub use registry_client::HttpAssetRegistry;
pub use service_token::{hash_token, PgServiceTokenRegistry};
pub use sleeper::TokioSleeper;
pub use vault::PgProviderSecrets;

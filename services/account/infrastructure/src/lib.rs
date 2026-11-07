//! Account infrastructure adapters: sqlx repositories, provider clients, Vault,
//! Auth admin, Audit, Notification, and JWT validation.

pub mod audit_client;
pub mod auth;
pub mod auth_admin;
pub mod chain;
pub mod clock;
pub mod db;
pub mod exchange;
pub mod http_retry;
pub mod migrations;
pub mod notification_client;
pub mod readiness;
pub mod registry_client;
pub mod repositories;
pub mod service_token;
pub mod vault;
pub mod wake;

pub use audit_client::HttpAuditClient;
pub use auth::{HttpJwksSource, JwkKey, JwksSource, JwksValidator};
pub use auth_admin::HttpAuthAdmin;
pub use chain::HttpChainProvider;
pub use clock::SystemClock;
pub use db::connect;
pub use exchange::{hmac_sha256_base64, hmac_sha256_hex, HttpExchangeProvider};
pub use notification_client::HttpNotificationClient;
pub use readiness::PgReadinessProbe;
pub use registry_client::HttpGetAssetRegistry;
pub use service_token::{hash_token, PgServiceTokenRegistry};
pub use vault::PgVaultClient;
pub use wake::HttpWake;

//! Identity infrastructure adapters.

pub mod audit_client;
pub mod auth;
pub mod auth_admin;
pub mod authorisation;
pub mod clients;
pub mod clock;
pub mod consent;
pub mod contributors;
pub mod db;
pub mod export_store;
pub mod gate_token;
pub mod http_retry;
pub mod migrations;
pub mod preferences;
pub mod readiness;
pub mod repositories;
pub mod service_token;
pub mod wake;

pub use audit_client::{HttpAuditClient, HttpDecisionAudit};
pub use auth::{HttpJwksSource, JwkKey, JwksSource, JwksValidator};
pub use auth_admin::HttpAuthAdmin;
pub use authorisation::audit::AuthorisationAuditSink;
pub use authorisation::role_writer::SupabaseRoleWriter;
pub use authorisation::status::StatusReaderLocal;
pub use authorisation::status_cache::StatusCache;
pub use clients::HttpGeoLocator;
pub use clock::SystemClock;
pub use contributors::HttpContributors;
pub use db::connect;
pub use export_store::SupabaseStorage;
pub use gate_token::Hs256GateTokens;
pub use readiness::PgReadinessProbe;
pub use service_token::FileServiceTokens;
pub use wake::HttpWake;

//! sqlx-backed repositories for the `portfolio` schema.

pub mod intents;
pub mod membership;
pub mod overlays;
pub mod portfolios;
pub mod privacy;
pub mod runs;
pub mod snapshots;
pub mod targets;
pub mod trades;

pub use intents::PgIntentRepository;
pub use membership::PgMembershipRepository;
pub use overlays::PgOverlayRepository;
pub use portfolios::PgPortfolioRepository;
pub use privacy::PgPrivacyRepository;
pub use runs::PgRunRepository;
pub use snapshots::PgSnapshotRepository;
pub use targets::PgTargetRepository;
pub use trades::PgTradeRepository;

use portfolio_application::ports::RepositoryError;

/// The `sqlx` SQLSTATE unique-violation code.
pub(crate) const UNIQUE_VIOLATION: &str = "23505";
/// The `sqlx` SQLSTATE insufficient-privilege code.
pub(crate) const INSUFFICIENT_PRIVILEGE: &str = "42501";

/// Maps a driver error without leaking driver types inward.
pub(crate) fn map_error(error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(database) = &error {
        if let Some(code) = database.code() {
            if code == UNIQUE_VIOLATION {
                return RepositoryError::Conflict("a uniqueness rule was violated".to_string());
            }
            if code == INSUFFICIENT_PRIVILEGE {
                return RepositoryError::Forbidden("the role lacks the grant".to_string());
            }
        }
    }
    RepositoryError::Unavailable(error.to_string())
}

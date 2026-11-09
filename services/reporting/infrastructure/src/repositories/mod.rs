//! sqlx-backed repositories for the `reporting` schema.

pub mod briefs;
pub mod deliveries;
pub mod privacy;
pub mod runs;

pub use briefs::PgBriefRepository;
pub use deliveries::PgDeliveryRepository;
pub use privacy::PgPrivacyRepository;
pub use runs::PgRunRepository;

use reporting_application::ports::RepositoryError;

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

/// Builds the invalid-data error for a stored row.
pub(crate) fn invalid_data(context: &str, detail: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::InvalidData(format!("{context}: {detail}"))
}

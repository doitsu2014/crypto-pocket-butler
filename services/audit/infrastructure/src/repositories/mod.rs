//! sqlx repositories implementing the application ports.
//!
//! Every query names the operation it performs so a refusal from the database —
//! `insufficient_privilege`, for instance — surfaces as a `Forbidden` with that
//! operation attached.

mod events;
mod exports;
mod privacy;
mod retention;
mod support;

pub use events::PgAuditEventRepository;
pub use exports::PgExportRepository;
pub use privacy::PgPrivacyDeletionRepository;
pub use retention::PgRetentionRepository;
pub use support::PgSupportSessionRepository;

use audit_application::ports::RepositoryError;

/// Maps a driver error to the inward-facing error type.
pub(crate) fn map_sqlx_error(operation: &str, error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(database) = &error {
        match database.code().as_deref() {
            Some("23505") => {
                return RepositoryError::Conflict(database.message().to_string());
            }
            Some("23503") => {
                return RepositoryError::Conflict(format!(
                    "referenced row does not exist: {}",
                    database.message()
                ));
            }
            Some("42501") => {
                return RepositoryError::Forbidden(format!(
                    "the database refused {operation}: {}",
                    database.message()
                ));
            }
            Some("55000") => {
                return RepositoryError::Conflict(database.message().to_string());
            }
            _ => {}
        }
    }
    RepositoryError::Unavailable(format!("{operation}: {error}"))
}

/// Maps a stored string back to a domain enum.
pub(crate) fn invalid_data(operation: &str, error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::InvalidData(format!("{operation}: {error}"))
}

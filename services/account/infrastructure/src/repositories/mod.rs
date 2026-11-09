//! sqlx repositories implementing the application ports.
//!
//! Every query names the operation it performs so a refusal from the database —
//! `insufficient_privilege`, for instance — surfaces as a `Forbidden` with that
//! operation attached.

mod accounts;
mod credentials;
mod holdings;
mod imports;
mod privacy;
mod sync_runs;
mod wallets;

pub use accounts::PgAccountRepository;
pub use credentials::PgCredentialRepository;
pub use holdings::PgHoldingRepository;
pub use imports::PgImportRepository;
pub use privacy::PgPrivacyRepository;
pub use sync_runs::PgSyncRunRepository;
pub use wallets::PgWalletConnectionRepository;

use account_application::ports::RepositoryError;

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
            Some("55000") | Some("23514") => {
                return RepositoryError::Conflict(database.message().to_string());
            }
            _ => {}
        }
    }
    RepositoryError::Unavailable(format!("{operation}: {error}"))
}

/// Maps a stored value back to a domain type.
pub(crate) fn invalid_data(operation: &str, error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::InvalidData(format!("{operation}: {error}"))
}

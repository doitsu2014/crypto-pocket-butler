//! sqlx repositories implementing the application ports.

mod anonymisations;
mod corrections;
mod lease;
mod lots;
mod snapshots;
mod subjects;
mod transactions;
mod transfer_pairs;

pub use anonymisations::PgAnonymisationRepository;
pub use corrections::PgCorrectionRepository;
pub use lease::PgReconcileLease;
pub use lots::PgLotRepository;
pub use snapshots::PgSnapshotRepository;
pub use subjects::PgSubjectRepository;
pub use transactions::PgTransactionRepository;
pub use transfer_pairs::PgTransferPairRepository;

use ledger_application::ports::RepositoryError;

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

/// Maps a stored value back to a domain type.
pub(crate) fn invalid_data(operation: &str, error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::InvalidData(format!("{operation}: {error}"))
}

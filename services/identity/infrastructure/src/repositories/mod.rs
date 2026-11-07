//! sqlx repositories implementing the application ports.
//!
//! Every query names the operation it performs so a refusal from the database —
//! `insufficient_privilege`, for instance — surfaces as a `Forbidden` with that
//! operation attached.

mod consent;
mod deletions;
mod exports;
mod jurisdiction;
mod keys;
mod sessions;
mod support;
mod users;

pub use consent::PgConsentRepository;
pub use deletions::PgDeletionRepository;
pub use exports::PgExportRepository;
pub use jurisdiction::PgJurisdictionRepository;
pub use keys::PgShreddingKeyRepository;
pub use sessions::PgDeviceSessionRepository;
pub use support::PgSupportGrantRepository;
pub use users::PgUserRepository;

use identity_application::ports::RepositoryError;
use sha2::{Digest, Sha256};

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

/// The SHA-256 hex digest used for the subject lookup index. The subject itself
/// is stored only as deterministic ciphertext.
pub(crate) fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

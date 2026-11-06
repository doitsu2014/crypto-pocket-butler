//! sqlx repositories implementing the application ports.

mod asset;
mod chain;
mod metadata;
mod observation;
mod price_source;

pub use asset::PgAssetRepository;
pub use chain::PgChainRepository;
pub use metadata::PgMetadataRepository;
pub use observation::PgObservationRepository;
pub use price_source::PgPriceSourceRepository;

use asset_registry_application::ports::RepositoryError;

/// Maps a driver error to the inward-facing error type.
pub(crate) fn map_sqlx_error(error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(database) = &error {
        if database.code().as_deref() == Some("23505") {
            return RepositoryError::Conflict(database.message().to_string());
        }
        if database.code().as_deref() == Some("23503") {
            return RepositoryError::Conflict(format!(
                "referenced row does not exist: {}",
                database.message()
            ));
        }
    }
    RepositoryError::Unavailable(error.to_string())
}

/// Converts a stored smallint precision into the domain's `u8`.
pub(crate) fn decimals_from_storage(value: i16) -> Result<u8, RepositoryError> {
    u8::try_from(value)
        .map_err(|_| RepositoryError::InvalidData(format!("stored decimals out of range: {value}")))
}

/// Converts a stored integer priority into the domain's `u32`.
pub(crate) fn priority_from_storage(value: i32) -> Result<u32, RepositoryError> {
    u32::try_from(value)
        .map_err(|_| RepositoryError::InvalidData(format!("stored priority out of range: {value}")))
}

//! The anonymiser gateway: one short-lived connection under the dedicated
//! `ledger_anonymiser` role, used only by the C7 anonymise handler (NFR9.2).

use async_trait::async_trait;
use ledger_application::ports::{AnonymiserGateway, RepositoryError};
use ledger_domain::LedgerSubjectId;

use crate::db::connect_once;

/// Maps a driver failure to the inward-facing error type.
fn map_driver_error(operation: &str, error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(database) = &error {
        if database.code().as_deref() == Some("42501") {
            return RepositoryError::Forbidden(format!(
                "the database refused {operation}: {}",
                database.message()
            ));
        }
    }
    RepositoryError::Unavailable(format!("{operation}: {error}"))
}

/// Opens a fresh connection per anonymisation and closes it immediately.
#[derive(Debug, Clone)]
pub struct PgAnonymiserGateway {
    database_url: String,
}

impl PgAnonymiserGateway {
    /// Creates the gateway from the anonymiser DSN (a secret file only).
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
        }
    }
}

#[async_trait]
impl AnonymiserGateway for PgAnonymiserGateway {
    async fn anonymise(
        &self,
        subject: LedgerSubjectId,
        anonymised_ref: &str,
    ) -> Result<(), RepositoryError> {
        let pool = connect_once(&self.database_url)
            .await
            .map_err(|error| map_driver_error("connect as ledger_anonymiser", error))?;
        let result = sqlx::query(
            "UPDATE ledger_subjects
             SET user_id = NULL, anonymised_ref = $2
             WHERE id = $1 AND user_id IS NOT NULL",
        )
        .bind(subject.as_uuid())
        .bind(anonymised_ref)
        .execute(&pool)
        .await;
        pool.close().await;
        result.map_err(|error| map_driver_error("anonymise the ledger subject", error))?;
        Ok(())
    }
}

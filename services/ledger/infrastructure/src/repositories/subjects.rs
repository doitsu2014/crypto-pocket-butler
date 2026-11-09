//! Postgres implementation of [`SubjectRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{RepositoryError, SubjectRepository};
use ledger_domain::{LedgerSubject, LedgerSubjectId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, user_id, anonymised_ref, created_at";

/// sqlx-backed mutable subject store.
#[derive(Debug, Clone)]
pub struct PgSubjectRepository {
    pool: PgPool,
}

impl PgSubjectRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SubjectRow {
    id: Uuid,
    user_id: Option<Uuid>,
    anonymised_ref: Option<String>,
    created_at: DateTime<Utc>,
}

impl SubjectRow {
    fn into_domain(self) -> LedgerSubject {
        LedgerSubject::from_stored(
            LedgerSubjectId::new(self.id),
            self.user_id.map(|value| value.to_string()),
            self.anonymised_ref,
            self.created_at,
        )
    }
}

#[async_trait]
impl SubjectRepository for PgSubjectRepository {
    async fn find_by_user(&self, user_id: &str) -> Result<Option<LedgerSubject>, RepositoryError> {
        let user = Uuid::parse_str(user_id)
            .map_err(|_| invalid_data("read subject", "userId is not a UUID"))?;
        let row = sqlx::query_as::<_, SubjectRow>(&format!(
            "SELECT {COLUMNS} FROM ledger_subjects WHERE user_id = $1"
        ))
        .bind(user)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find subject by user", error))?;
        Ok(row.map(SubjectRow::into_domain))
    }

    async fn find_by_id(
        &self,
        id: LedgerSubjectId,
    ) -> Result<Option<LedgerSubject>, RepositoryError> {
        let row = sqlx::query_as::<_, SubjectRow>(&format!(
            "SELECT {COLUMNS} FROM ledger_subjects WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find subject", error))?;
        Ok(row.map(SubjectRow::into_domain))
    }

    async fn create(&self, subject: &LedgerSubject) -> Result<(), RepositoryError> {
        let user = subject
            .user_id()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| invalid_data("insert subject", "userId is not a UUID"))?;
        sqlx::query(
            "INSERT INTO ledger_subjects (id, user_id, anonymised_ref, created_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(subject.id().as_uuid())
        .bind(user)
        .bind(subject.anonymised_ref())
        .bind(subject.created_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert subject", error))?;
        Ok(())
    }

    async fn list_ids(&self) -> Result<Vec<LedgerSubjectId>, RepositoryError> {
        let ids = sqlx::query_scalar::<_, Uuid>("SELECT id FROM ledger_subjects ORDER BY id")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("list subjects", error))?;
        Ok(ids.into_iter().map(LedgerSubjectId::new).collect())
    }
}

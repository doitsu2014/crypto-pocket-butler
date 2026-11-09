//! The portfolios repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{PortfolioRepository, RepositoryError};
use portfolio_domain::{Portfolio, PortfolioId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{map_error, UNIQUE_VIOLATION};

/// sqlx-backed portfolios.
#[derive(Debug, Clone)]
pub struct PgPortfolioRepository {
    pool: PgPool,
}

impl PgPortfolioRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PortfolioRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    base_currency: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    anonymised_at: Option<DateTime<Utc>>,
}

impl From<PortfolioRow> for Portfolio {
    fn from(row: PortfolioRow) -> Self {
        Portfolio::from_stored(
            PortfolioId::new(row.id),
            UserId::new(row.user_id),
            row.name,
            row.base_currency,
            row.created_at,
            row.updated_at,
            row.anonymised_at,
        )
    }
}

const SELECT: &str = "SELECT id, user_id, name, base_currency, created_at, updated_at, anonymised_at FROM portfolios";

#[async_trait]
impl PortfolioRepository for PgPortfolioRepository {
    async fn insert(&self, portfolio: &Portfolio) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO portfolios
                (id, user_id, name, base_currency, created_at, updated_at, anonymised_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(portfolio.id().as_uuid())
        .bind(portfolio.user_id().as_uuid())
        .bind(portfolio.name())
        .bind(portfolio.base_currency())
        .bind(portfolio.created_at())
        .bind(portfolio.updated_at())
        .bind(portfolio.anonymised_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| match &error {
            sqlx::Error::Database(database)
                if database.code().as_deref() == Some(UNIQUE_VIOLATION) =>
            {
                RepositoryError::Conflict("the name is already used".to_string())
            }
            _ => map_error(error),
        })
    }

    async fn find_by_id(&self, id: PortfolioId) -> Result<Option<Portfolio>, RepositoryError> {
        let row = sqlx::query_as::<_, PortfolioRow>(&format!("{SELECT} WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_error)?;
        Ok(row.map(Portfolio::from))
    }

    async fn find_for_user(&self, user_id: UserId) -> Result<Vec<Portfolio>, RepositoryError> {
        let rows = sqlx::query_as::<_, PortfolioRow>(&format!(
            "{SELECT} WHERE user_id = $1 ORDER BY created_at"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(Portfolio::from).collect())
    }

    async fn update(&self, portfolio: &Portfolio) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE portfolios
             SET name = $2, base_currency = $3, updated_at = $4, anonymised_at = $5
             WHERE id = $1",
        )
        .bind(portfolio.id().as_uuid())
        .bind(portfolio.name())
        .bind(portfolio.base_currency())
        .bind(portfolio.updated_at())
        .bind(portfolio.anonymised_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }

    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "UPDATE portfolios
             SET user_id = $2,
                 name = 'Anonymised portfolio ' || left(id::text, 8),
                 anonymised_at = $3,
                 updated_at = $3
             WHERE user_id = $1 AND anonymised_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .bind(anonymised_ref.as_uuid())
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected())
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Portfolio>, RepositoryError> {
        let rows = sqlx::query_as::<_, PortfolioRow>(&format!(
            "{SELECT} ORDER BY created_at, id LIMIT $1 OFFSET $2"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(Portfolio::from).collect())
    }
}

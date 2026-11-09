//! The account-membership repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{MembershipRepository, RepositoryError};
use portfolio_domain::{AccountId, PortfolioAccount, PortfolioAccountId, PortfolioId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::map_error;

/// sqlx-backed membership.
#[derive(Debug, Clone)]
pub struct PgMembershipRepository {
    pool: PgPool,
}

impl PgMembershipRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct MembershipRow {
    id: Uuid,
    portfolio_id: Uuid,
    account_id: Uuid,
    added_at: DateTime<Utc>,
}

impl From<MembershipRow> for PortfolioAccount {
    fn from(row: MembershipRow) -> Self {
        PortfolioAccount::from_stored(
            PortfolioAccountId::new(row.id),
            PortfolioId::new(row.portfolio_id),
            AccountId::new(row.account_id),
            row.added_at,
        )
    }
}

const SELECT: &str = "SELECT id, portfolio_id, account_id, added_at FROM portfolio_accounts";

#[async_trait]
impl MembershipRepository for PgMembershipRepository {
    async fn assign(&self, membership: &PortfolioAccount) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO portfolio_accounts (id, portfolio_id, account_id, added_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (account_id) DO UPDATE
             SET portfolio_id = EXCLUDED.portfolio_id, added_at = EXCLUDED.added_at",
        )
        .bind(membership.id().as_uuid())
        .bind(membership.portfolio_id().as_uuid())
        .bind(membership.account_id().as_uuid())
        .bind(membership.added_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<PortfolioAccount>, RepositoryError> {
        let rows = sqlx::query_as::<_, MembershipRow>(&format!(
            "{SELECT} WHERE portfolio_id = $1 ORDER BY added_at"
        ))
        .bind(portfolio_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(PortfolioAccount::from).collect())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PortfolioAccount>, RepositoryError> {
        let rows = sqlx::query_as::<_, MembershipRow>(&format!(
            "{SELECT} WHERE portfolio_id IN (SELECT id FROM portfolios WHERE user_id = $1)
             ORDER BY added_at"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(PortfolioAccount::from).collect())
    }
}

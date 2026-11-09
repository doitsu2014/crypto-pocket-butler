//! Postgres implementation of [`WalletConnectionRepository`].

use account_application::ports::{RepositoryError, WalletConnectionRepository};
use account_domain::{AccountId, UserId, WalletConnection, WalletConnectionId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_error;

const COLUMNS: &str = "id, account_id, address, observed_chain, session_ref, connected_at";

/// sqlx-backed wallet connections.
#[derive(Debug, Clone)]
pub struct PgWalletConnectionRepository {
    pool: PgPool,
}

impl PgWalletConnectionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct WalletRow {
    id: Uuid,
    account_id: Uuid,
    address: String,
    observed_chain: String,
    session_ref: Option<String>,
    connected_at: DateTime<Utc>,
}

impl WalletRow {
    fn into_domain(self) -> WalletConnection {
        WalletConnection::from_stored(
            WalletConnectionId::new(self.id),
            AccountId::new(self.account_id),
            self.address,
            self.observed_chain,
            self.session_ref,
            self.connected_at,
        )
    }
}

#[async_trait]
impl WalletConnectionRepository for PgWalletConnectionRepository {
    async fn insert(&self, connection: &WalletConnection) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO wallet_connections
                 (id, account_id, address, observed_chain, session_ref, connected_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(connection.id().as_uuid())
        .bind(connection.account_id().as_uuid())
        .bind(connection.address())
        .bind(connection.observed_chain())
        .bind(connection.session_ref())
        .bind(connection.connected_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert wallet connection", error))?;
        Ok(())
    }

    async fn find_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<WalletConnection>, RepositoryError> {
        let row = sqlx::query_as::<_, WalletRow>(&format!(
            "SELECT {COLUMNS} FROM wallet_connections WHERE account_id = $1"
        ))
        .bind(account_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find wallet connection", error))?;
        Ok(row.map(WalletRow::into_domain))
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<WalletConnection>, RepositoryError> {
        let rows = sqlx::query_as::<_, WalletRow>(
            "SELECT w.id, w.account_id, w.address, w.observed_chain, w.session_ref, w.connected_at
             FROM wallet_connections w
             JOIN accounts a ON a.id = w.account_id
             WHERE a.user_id = $1
             ORDER BY w.connected_at, w.id",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list user wallet connections", error))?;
        Ok(rows.into_iter().map(WalletRow::into_domain).collect())
    }

    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        Ok(sqlx::query(
            "DELETE FROM wallet_connections w
             USING accounts a
             WHERE w.account_id = a.id AND a.user_id = $1",
        )
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("delete user wallet connections", error))?
        .rows_affected())
    }
}

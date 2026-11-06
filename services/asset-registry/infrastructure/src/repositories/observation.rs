//! Postgres implementation of [`ObservationRepository`].

use asset_registry_application::ports::{ObservationRepository, RepositoryError};
use asset_registry_domain::{
    AssetId, ChainId, ContractAddress, ObservationId, ObservationStatus, UnmappedObservation,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::map_sqlx_error;

/// sqlx-backed quarantine repository.
#[derive(Debug, Clone)]
pub struct PgObservationRepository {
    pool: PgPool,
}

impl PgObservationRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ObservationRow {
    id: String,
    observed_symbol: String,
    chain_id: String,
    contract_address: Option<String>,
    first_seen_at: DateTime<Utc>,
    status: String,
    resolved_asset_id: Option<String>,
    ignore_reason: Option<String>,
}

impl ObservationRow {
    fn into_domain(self) -> Result<UnmappedObservation, RepositoryError> {
        let chain = ChainId::new(self.chain_id)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        let address = self
            .contract_address
            .map(|raw| {
                ContractAddress::parse(&raw)
                    .map_err(|error| RepositoryError::InvalidData(error.to_string()))
            })
            .transpose()?;
        let mut observation = UnmappedObservation::quarantine(
            self.id,
            &self.observed_symbol,
            &chain,
            address,
            self.first_seen_at,
        )
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;

        match self.status.as_str() {
            "quarantined" => {}
            "resolved" => {
                let asset_id = self.resolved_asset_id.ok_or_else(|| {
                    RepositoryError::InvalidData(
                        "resolved observation has no resolved_asset_id".to_string(),
                    )
                })?;
                let asset_id = AssetId::new(asset_id)
                    .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
                observation
                    .resolve(asset_id)
                    .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
            }
            "ignored" => {
                let reason = self.ignore_reason.ok_or_else(|| {
                    RepositoryError::InvalidData(
                        "ignored observation has no ignore_reason".to_string(),
                    )
                })?;
                observation
                    .ignore(&reason)
                    .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
            }
            other => {
                return Err(RepositoryError::InvalidData(format!(
                    "unknown observation status '{other}'"
                )));
            }
        }
        Ok(observation)
    }
}

const SELECT_COLUMNS: &str = "id, observed_symbol, chain_id, contract_address, first_seen_at, \
                              status, resolved_asset_id, ignore_reason";

fn status_fields(status: &ObservationStatus) -> (String, Option<String>, Option<String>) {
    match status {
        ObservationStatus::Quarantined => ("quarantined".to_string(), None, None),
        ObservationStatus::Resolved { asset_id } => (
            "resolved".to_string(),
            Some(asset_id.as_str().to_string()),
            None,
        ),
        ObservationStatus::Ignored { reason } => {
            ("ignored".to_string(), None, Some(reason.clone()))
        }
    }
}

#[async_trait]
impl ObservationRepository for PgObservationRepository {
    async fn find_by_token(
        &self,
        chain: &ChainId,
        observed_symbol: &str,
        contract_key: Option<&str>,
    ) -> Result<Option<UnmappedObservation>, RepositoryError> {
        let row = sqlx::query_as::<_, ObservationRow>(&format!(
            "SELECT {SELECT_COLUMNS}
             FROM unmapped_observations
             WHERE chain_id = $1
               AND observed_symbol = $2
               AND COALESCE(contract_address_key, '') = COALESCE($3, '')"
        ))
        .bind(chain.as_str())
        .bind(observed_symbol)
        .bind(contract_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.map(ObservationRow::into_domain).transpose()
    }

    async fn find_by_id(
        &self,
        id: &ObservationId,
    ) -> Result<Option<UnmappedObservation>, RepositoryError> {
        let row = sqlx::query_as::<_, ObservationRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM unmapped_observations WHERE id = $1"
        ))
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.map(ObservationRow::into_domain).transpose()
    }

    async fn insert(
        &self,
        observation: &UnmappedObservation,
        contract_key: Option<&str>,
    ) -> Result<(), RepositoryError> {
        let (status, resolved_asset_id, ignore_reason) = status_fields(observation.status());
        sqlx::query(
            "INSERT INTO unmapped_observations
                 (id, observed_symbol, chain_id, contract_address, contract_address_key,
                  first_seen_at, status, resolved_asset_id, ignore_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(observation.id().as_str())
        .bind(observation.observed_symbol())
        .bind(observation.chain_id().as_str())
        .bind(observation.contract_address().map(ContractAddress::as_str))
        .bind(contract_key)
        .bind(observation.first_seen_at())
        .bind(status)
        .bind(resolved_asset_id)
        .bind(ignore_reason)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn save_status(&self, observation: &UnmappedObservation) -> Result<(), RepositoryError> {
        let (status, resolved_asset_id, ignore_reason) = status_fields(observation.status());
        let result = sqlx::query(
            "UPDATE unmapped_observations
             SET status = $1, resolved_asset_id = $2, ignore_reason = $3, updated_at = now()
             WHERE id = $4",
        )
        .bind(status)
        .bind(resolved_asset_id)
        .bind(ignore_reason)
        .bind(observation.id().as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::Conflict(format!(
                "observation '{}' does not exist",
                observation.id()
            )));
        }
        Ok(())
    }

    async fn list_by_status(
        &self,
        status: &ObservationStatus,
    ) -> Result<Vec<UnmappedObservation>, RepositoryError> {
        let rows = sqlx::query_as::<_, ObservationRow>(&format!(
            "SELECT {SELECT_COLUMNS}
             FROM unmapped_observations
             WHERE status = $1
             ORDER BY first_seen_at ASC"
        ))
        .bind(status.label())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(ObservationRow::into_domain).collect()
    }
}

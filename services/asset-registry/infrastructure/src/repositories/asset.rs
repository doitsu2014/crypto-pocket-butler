//! Postgres implementation of [`AssetRepository`].

use asset_registry_application::ports::{AssetRepository, RepositoryError};
use asset_registry_domain::{Asset, AssetId, ChainFamily, ChainId, ContractAddress, TokenContract};
use async_trait::async_trait;
use sqlx::PgPool;

use super::{decimals_from_storage, map_sqlx_error};

/// sqlx-backed asset and token-contract repository.
#[derive(Debug, Clone)]
pub struct PgAssetRepository {
    pool: PgPool,
}

impl PgAssetRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AssetRow {
    id: String,
    symbol: String,
    name: String,
    decimals: i16,
    canonical: bool,
}

impl AssetRow {
    fn into_domain(self) -> Result<Asset, RepositoryError> {
        Asset::new(
            self.id,
            &self.symbol,
            &self.name,
            decimals_from_storage(self.decimals)?,
            self.canonical,
        )
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))
    }
}

const SELECT_ASSET_COLUMNS: &str = "a.id, a.symbol, a.name, a.decimals, a.canonical";

#[async_trait]
impl AssetRepository for PgAssetRepository {
    async fn find_asset_by_contract(
        &self,
        chain: &ChainId,
        family: ChainFamily,
        address: &ContractAddress,
    ) -> Result<Option<Asset>, RepositoryError> {
        let row = sqlx::query_as::<_, AssetRow>(&format!(
            "SELECT {SELECT_ASSET_COLUMNS}
             FROM token_contracts tc
             JOIN assets a ON a.id = tc.asset_id
             WHERE tc.chain_id = $1 AND tc.contract_address_key = $2"
        ))
        .bind(chain.as_str())
        .bind(address.key_for(family))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.map(AssetRow::into_domain).transpose()
    }

    async fn find_native_asset(
        &self,
        chain: &ChainId,
        symbol: &str,
    ) -> Result<Option<Asset>, RepositoryError> {
        let row = sqlx::query_as::<_, AssetRow>(&format!(
            "SELECT {SELECT_ASSET_COLUMNS}
             FROM native_assets n
             JOIN assets a ON a.id = n.asset_id
             WHERE n.chain_id = $1 AND n.symbol = $2"
        ))
        .bind(chain.as_str())
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.map(AssetRow::into_domain).transpose()
    }

    async fn find_asset_by_id(&self, id: &AssetId) -> Result<Option<Asset>, RepositoryError> {
        let row = sqlx::query_as::<_, AssetRow>(&format!(
            "SELECT {SELECT_ASSET_COLUMNS} FROM assets a WHERE a.id = $1"
        ))
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.map(AssetRow::into_domain).transpose()
    }

    async fn insert_asset(
        &self,
        asset: &Asset,
        native_chain: Option<&ChainId>,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO assets (id, symbol, name, decimals, canonical)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(asset.id().as_str())
        .bind(asset.symbol())
        .bind(asset.name())
        .bind(i16::from(asset.decimals()))
        .bind(asset.canonical())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        if let Some(chain) = native_chain {
            sqlx::query(
                "INSERT INTO native_assets (chain_id, asset_id, symbol)
                 VALUES ($1, $2, $3)",
            )
            .bind(chain.as_str())
            .bind(asset.id().as_str())
            .bind(asset.symbol())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }

        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn insert_token_contract(&self, contract: &TokenContract) -> Result<(), RepositoryError> {
        // The chain family decides the lookup key: EVM/BNB lower-case, Solana exact.
        let result = sqlx::query(
            "INSERT INTO token_contracts
                 (id, chain_id, contract_address, contract_address_key, asset_id, decimals)
             SELECT $1, c.id, $2,
                    CASE c.family WHEN 'solana' THEN $2 ELSE lower($2) END,
                    $3, $4
             FROM chains c
             WHERE c.id = $5",
        )
        .bind(contract.id().as_str())
        .bind(contract.contract_address().as_str())
        .bind(contract.asset_id().as_str())
        .bind(i16::from(contract.decimals()))
        .bind(contract.chain_id().as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::Conflict(format!(
                "chain '{}' is not registered",
                contract.chain_id()
            )));
        }
        Ok(())
    }
}

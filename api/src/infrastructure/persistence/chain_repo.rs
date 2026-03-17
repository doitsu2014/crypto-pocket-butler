/// SeaORM-backed implementation of `ChainRepository`.
///
/// Translates between the `evm_chains` SeaORM entity and the `EvmChain`
/// domain aggregate. Solana-token persistence will be added in a future PR.

use async_trait::async_trait;
use chrono::DateTime;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domains::chain::{
    aggregate::{ChainError, EvmChain},
    entities::SolanaToken,
    repository::ChainRepository,
};
use crate::infrastructure::persistence::entities::evm_chains;

/// SeaORM-backed implementation of [`ChainRepository`].
pub struct ChainRepositoryImpl {
    db: DatabaseConnection,
}

impl ChainRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn to_domain(model: evm_chains::Model) -> EvmChain {
        EvmChain::from_persistence(
            model.id,
            model.chain_id,
            model.name,
            model.rpc_url,
            model.native_symbol,
            model.is_active,
            DateTime::from(model.created_at),
            DateTime::from(model.updated_at),
            vec![],
        )
    }
}

#[async_trait]
impl ChainRepository for ChainRepositoryImpl {
    async fn find_chain_by_id(&self, id: Uuid) -> Result<Option<EvmChain>, ChainError> {
        let model = evm_chains::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ChainError::PersistenceError(e.to_string()))?;
        Ok(model.map(Self::to_domain))
    }

    async fn find_chain_by_chain_id(
        &self,
        chain_id: &str,
    ) -> Result<Option<EvmChain>, ChainError> {
        let model = evm_chains::Entity::find()
            .filter(evm_chains::Column::ChainId.eq(chain_id))
            .one(&self.db)
            .await
            .map_err(|e| ChainError::PersistenceError(e.to_string()))?;
        Ok(model.map(Self::to_domain))
    }

    async fn find_all_chains(&self) -> Result<Vec<EvmChain>, ChainError> {
        let models = evm_chains::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| ChainError::PersistenceError(e.to_string()))?;
        Ok(models.into_iter().map(Self::to_domain).collect())
    }

    async fn find_active_chains(&self) -> Result<Vec<EvmChain>, ChainError> {
        let models = evm_chains::Entity::find()
            .filter(evm_chains::Column::IsActive.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| ChainError::PersistenceError(e.to_string()))?;
        Ok(models.into_iter().map(Self::to_domain).collect())
    }

    async fn save_chain(&self, chain: &EvmChain) -> Result<(), ChainError> {
        use sea_orm::Set;

        let active_model = evm_chains::ActiveModel {
            id: Set(chain.id),
            chain_id: Set(chain.chain_id.clone()),
            name: Set(chain.name.clone()),
            rpc_url: Set(chain.rpc_url.clone()),
            native_symbol: Set(chain.native_symbol.clone()),
            is_active: Set(chain.is_active),
            created_at: Set(chain.created_at.into()),
            updated_at: Set(chain.updated_at.into()),
        };

        let _ = evm_chains::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(evm_chains::Column::Id)
                    .update_columns([
                        evm_chains::Column::Name,
                        evm_chains::Column::RpcUrl,
                        evm_chains::Column::NativeSymbol,
                        evm_chains::Column::IsActive,
                        evm_chains::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| ChainError::PersistenceError(e.to_string()))?;
        Ok(())
    }

    async fn delete_chain(&self, id: Uuid) -> Result<bool, ChainError> {
        let result = evm_chains::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| ChainError::PersistenceError(e.to_string()))?;
        Ok(result.rows_affected > 0)
    }

    // ─── Solana tokens ───────────────────────────────────────────────────────────

    async fn find_all_solana_tokens(&self) -> Result<Vec<SolanaToken>, ChainError> {
        // Solana token persistence is handled separately by the solana_tokens handler.
        // Return an empty vec here until a dedicated SolanaToken repository is wired up.
        tracing::warn!("ChainRepositoryImpl::find_all_solana_tokens is not yet implemented; returning empty list");
        Ok(vec![])
    }

    async fn save_solana_token(&self, _token: &SolanaToken) -> Result<(), ChainError> {
        tracing::warn!("ChainRepositoryImpl::save_solana_token is not yet implemented; operation is a no-op");
        Ok(())
    }

    async fn delete_solana_token(&self, _id: Uuid) -> Result<bool, ChainError> {
        tracing::warn!("ChainRepositoryImpl::delete_solana_token is not yet implemented; returning false");
        Ok(false)
    }
}

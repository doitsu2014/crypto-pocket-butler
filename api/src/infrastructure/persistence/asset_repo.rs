/// SeaORM-backed implementation of `AssetRepository`.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::domains::asset::{
    aggregate::{Asset, AssetError},
    repository::AssetRepository,
};
use crate::infrastructure::persistence::entities::assets;

/// SeaORM-backed implementation of [`AssetRepository`].
pub struct AssetRepositoryImpl {
    db: DatabaseConnection,
}

impl AssetRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn to_domain(model: assets::Model) -> Asset {
        Asset::from_persistence(
            model.symbol,
            model.name,
            model.coinpaprika_id,
            None,  // rank loaded from asset_prices separately
            model.is_active,
            None,  // price loaded separately
            vec![], // contracts loaded separately
        )
    }
}

#[async_trait]
impl AssetRepository for AssetRepositoryImpl {
    async fn find_by_symbol(&self, symbol: &str) -> Result<Option<Asset>, AssetError> {
        let model = assets::Entity::find()
            .filter(assets::Column::Symbol.eq(symbol))
            .one(&self.db)
            .await
            .map_err(|e| AssetError::PersistenceError(e.to_string()))?;
        Ok(model.map(Self::to_domain))
    }

    async fn find_by_symbol_and_name(
        &self,
        symbol: &str,
        name: &str,
    ) -> Result<Option<Asset>, AssetError> {
        let model = assets::Entity::find()
            .filter(assets::Column::Symbol.eq(symbol))
            .filter(assets::Column::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| AssetError::PersistenceError(e.to_string()))?;
        Ok(model.map(Self::to_domain))
    }

    async fn find_by_coinpaprika_id(&self, id: &str) -> Result<Option<Asset>, AssetError> {
        let model = assets::Entity::find()
            .filter(assets::Column::CoinpaprikaId.eq(id))
            .one(&self.db)
            .await
            .map_err(|e| AssetError::PersistenceError(e.to_string()))?;
        Ok(model.map(Self::to_domain))
    }

    async fn find_all(&self) -> Result<Vec<Asset>, AssetError> {
        let models = assets::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| AssetError::PersistenceError(e.to_string()))?;
        Ok(models.into_iter().map(Self::to_domain).collect())
    }

    async fn save(&self, asset: &Asset) -> Result<(), AssetError> {
        use sea_orm::Set;

        let active_model = assets::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            symbol: Set(asset.symbol.clone()),
            name: Set(asset.name.clone()),
            asset_type: Set("cryptocurrency".to_string()),
            coinpaprika_id: Set(asset.coinpaprika_id.clone()),
            coinmarketcap_id: Set(None),
            logo_url: Set(None),
            description: Set(None),
            decimals: Set(None),
            is_active: Set(asset.is_active),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let _ = assets::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::columns([
                    assets::Column::Symbol,
                    assets::Column::Name,
                ])
                .update_columns([
                    assets::Column::CoinpaprikaId,
                    assets::Column::IsActive,
                    assets::Column::UpdatedAt,
                ])
                .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| AssetError::PersistenceError(e.to_string()))?;
        Ok(())
    }
}

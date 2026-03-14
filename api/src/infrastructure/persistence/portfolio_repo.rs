/// SeaORM-backed implementation of `PortfolioRepository`.

use async_trait::async_trait;
use chrono::DateTime;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domains::portfolio::{
    aggregate::{Portfolio, PortfolioError},
    repository::PortfolioRepository,
    value_objects::{Guardrails, TargetAllocation},
};
use crate::infrastructure::persistence::entities::portfolios;

/// SeaORM-backed implementation of [`PortfolioRepository`].
pub struct PortfolioRepositoryImpl {
    db: DatabaseConnection,
}

impl PortfolioRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn to_domain(model: portfolios::Model) -> Result<Portfolio, PortfolioError> {
        let target_allocation: TargetAllocation = model
            .target_allocation
            .as_ref()
            .and_then(|j| serde_json::from_value(j.clone()).ok())
            .unwrap_or_default();

        let guardrails: Option<Guardrails> = model
            .guardrails
            .as_ref()
            .and_then(|j| serde_json::from_value(j.clone()).ok());

        Ok(Portfolio::from_persistence(
            model.id,
            model.user_id,
            model.name,
            model.description,
            model.is_default,
            target_allocation,
            guardrails,
            model.last_constructed_at.map(DateTime::from),
            DateTime::from(model.created_at),
            DateTime::from(model.updated_at),
            vec![], // accounts loaded separately via portfolio_accounts
        ))
    }
}

#[async_trait]
impl PortfolioRepository for PortfolioRepositoryImpl {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Portfolio>, PortfolioError> {
        let model = portfolios::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PortfolioError::PersistenceError(e.to_string()))?;
        model.map(Self::to_domain).transpose()
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Portfolio>, PortfolioError> {
        let models = portfolios::Entity::find()
            .filter(portfolios::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
            .map_err(|e| PortfolioError::PersistenceError(e.to_string()))?;
        models.into_iter().map(Self::to_domain).collect()
    }

    async fn find_default_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Portfolio>, PortfolioError> {
        let model = portfolios::Entity::find()
            .filter(portfolios::Column::UserId.eq(user_id))
            .filter(portfolios::Column::IsDefault.eq(true))
            .one(&self.db)
            .await
            .map_err(|e| PortfolioError::PersistenceError(e.to_string()))?;
        model.map(Self::to_domain).transpose()
    }

    async fn save(&self, portfolio: &Portfolio) -> Result<(), PortfolioError> {
        use sea_orm::Set;

        let target_alloc_json = serde_json::to_value(&portfolio.target_allocation)
            .map_err(|e| PortfolioError::PersistenceError(e.to_string()))?;
        let guardrails_json = portfolio
            .guardrails
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| PortfolioError::PersistenceError(e.to_string()))?;

        let active_model = portfolios::ActiveModel {
            id: Set(portfolio.id),
            user_id: Set(portfolio.user_id),
            name: Set(portfolio.name.clone()),
            description: Set(portfolio.description.clone()),
            is_default: Set(portfolio.is_default),
            target_allocation: Set(Some(target_alloc_json)),
            guardrails: Set(guardrails_json),
            last_constructed_at: Set(portfolio.last_constructed_at.map(Into::into)),
            created_at: Set(portfolio.created_at.into()),
            updated_at: Set(portfolio.updated_at.into()),
        };

        let _ = portfolios::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(portfolios::Column::Id)
                    .update_columns([
                        portfolios::Column::Name,
                        portfolios::Column::Description,
                        portfolios::Column::IsDefault,
                        portfolios::Column::TargetAllocation,
                        portfolios::Column::Guardrails,
                        portfolios::Column::LastConstructedAt,
                        portfolios::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| PortfolioError::PersistenceError(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool, PortfolioError> {
        let result = portfolios::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| PortfolioError::PersistenceError(e.to_string()))?;
        Ok(result.rows_affected > 0)
    }
}

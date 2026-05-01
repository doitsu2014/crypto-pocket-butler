/// SeaORM-backed implementation of `AccountRepository`.
///
/// Translates between the `accounts` SeaORM entity and the `Account` domain aggregate.

use async_trait::async_trait;
use chrono::DateTime;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domains::account::{
    aggregate::{Account, AccountError},
    entities::{AccountHolding, AccountHoldings},
    repository::AccountRepository,
    value_objects::{AccountCredentials, AccountType},
};
use crate::infrastructure::persistence::entities::accounts;

/// SeaORM-backed implementation of [`AccountRepository`].
pub struct AccountRepositoryImpl {
    db: DatabaseConnection,
}

impl AccountRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Map a SeaORM `accounts::Model` to the `Account` domain aggregate.
    fn to_domain(model: accounts::Model) -> Result<Account, AccountError> {
        let account_type = AccountType::from_str(&model.account_type)
            .ok_or_else(|| AccountError::PersistenceError(
                format!("unknown account_type: {}", model.account_type),
            ))?;

        let credentials = match (&model.api_key_encrypted, &model.api_secret_encrypted) {
            (Some(key), Some(secret)) if !key.is_empty() && !secret.is_empty() => {
                Some(AccountCredentials::new(
                    key.clone(),
                    secret.clone(),
                    model.passphrase_encrypted.clone(),
                ))
            }
            _ => None,
        };

        let holdings_items: Vec<AccountHolding> = model
            .holdings
            .as_ref()
            .and_then(|j| serde_json::from_value(j.clone()).ok())
            .unwrap_or_default();

        let enabled_chains: Vec<String> = model
            .enabled_chains
            .as_ref()
            .and_then(|j| serde_json::from_value(j.clone()).ok())
            .unwrap_or_default();

        let last_synced_at = model.last_synced_at.map(|dt| {
            DateTime::from(dt)
        });

        Ok(Account::from_persistence(
            model.id,
            model.user_id,
            model.name,
            account_type,
            model.is_active,
            last_synced_at,
            DateTime::from(model.created_at),
            DateTime::from(model.updated_at),
            model.exchange_name,
            credentials,
            model.wallet_address,
            enabled_chains,
            AccountHoldings::new(holdings_items),
        ))
    }
}

#[async_trait]
impl AccountRepository for AccountRepositoryImpl {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, AccountError> {
        let model = accounts::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| AccountError::PersistenceError(e.to_string()))?;
        model.map(Self::to_domain).transpose()
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Account>, AccountError> {
        let models = accounts::Entity::find()
            .filter(accounts::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
            .map_err(|e| AccountError::PersistenceError(e.to_string()))?;
        models.into_iter().map(Self::to_domain).collect()
    }

    async fn save(&self, account: &Account) -> Result<(), AccountError> {
        use sea_orm::Set;

        let holdings_json = serde_json::to_value(account.holdings().items.clone())
            .map_err(|e| AccountError::PersistenceError(e.to_string()))?;
        let enabled_chains_json = serde_json::to_value(account.enabled_chains.clone())
            .map_err(|e| AccountError::PersistenceError(e.to_string()))?;

        let active_model = accounts::ActiveModel {
            id: Set(account.id),
            user_id: Set(account.user_id),
            name: Set(account.name.clone()),
            account_type: Set(account.account_type.as_str().to_string()),
            exchange_name: Set(account.exchange_name.clone()),
            api_key_encrypted: Set(
                account.credentials.as_ref().map(|c| c.api_key_encrypted.clone())
            ),
            api_secret_encrypted: Set(
                account.credentials.as_ref().map(|c| c.api_secret_encrypted.clone())
            ),
            passphrase_encrypted: Set(
                account.credentials.as_ref().and_then(|c| c.passphrase_encrypted.clone())
            ),
            wallet_address: Set(account.wallet_address.clone()),
            is_active: Set(account.is_active),
            last_synced_at: Set(account.last_synced_at.map(Into::into)),
            holdings: Set(Some(holdings_json)),
            enabled_chains: Set(Some(enabled_chains_json)),
            created_at: Set(account.created_at.into()),
            updated_at: Set(account.updated_at.into()),
        };

        let _ = accounts::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(accounts::Column::Id)
                    .update_columns([
                        accounts::Column::Name,
                        accounts::Column::IsActive,
                        accounts::Column::LastSyncedAt,
                        accounts::Column::Holdings,
                        accounts::Column::EnabledChains,
                        accounts::Column::ApiKeyEncrypted,
                        accounts::Column::ApiSecretEncrypted,
                        accounts::Column::PassphraseEncrypted,
                        accounts::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| AccountError::PersistenceError(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool, AccountError> {
        let result = accounts::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| AccountError::PersistenceError(e.to_string()))?;
        Ok(result.rows_affected > 0)
    }

    async fn find_active_by_type(
        &self,
        account_type: AccountType,
    ) -> Result<Vec<Account>, AccountError> {
        let models = accounts::Entity::find()
            .filter(accounts::Column::AccountType.eq(account_type.as_str()))
            .filter(accounts::Column::IsActive.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| AccountError::PersistenceError(e.to_string()))?;
        models.into_iter().map(Self::to_domain).collect()
    }
}

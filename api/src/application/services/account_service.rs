/// AccountService — application service for account use cases.
///
/// Coordinates domain logic, repository access, and credential encryption.
/// The HTTP handlers call this service; domain types never see HTTP concerns.

use std::sync::Arc;
use uuid::Uuid;

use crate::domains::account::{
    aggregate::{Account, AccountError},
    entities::AccountHolding,
    repository::AccountRepository,
    value_objects::AccountCredentials,
};

/// Application service for account operations.
pub struct AccountService {
    repo: Arc<dyn AccountRepository>,
}

impl AccountService {
    pub fn new(repo: Arc<dyn AccountRepository>) -> Self {
        Self { repo }
    }

    /// Create a new exchange account with encrypted credentials.
    ///
    /// Callers are responsible for encrypting credentials before calling this.
    pub async fn create_exchange_account(
        &self,
        user_id: Uuid,
        name: String,
        exchange_name: String,
        api_key_encrypted: String,
        api_secret_encrypted: String,
        passphrase_encrypted: Option<String>,
    ) -> Result<Account, AccountError> {
        let id = Uuid::new_v4();
        let credentials = AccountCredentials::new(
            api_key_encrypted,
            api_secret_encrypted,
            passphrase_encrypted,
        );
        let account = Account::new_exchange(id, user_id, name, exchange_name, credentials)?;
        self.repo.save(&account).await?;
        Ok(account)
    }

    /// Create a new wallet account.
    pub async fn create_wallet_account(
        &self,
        user_id: Uuid,
        name: String,
        wallet_address: String,
        enabled_chains: Vec<String>,
    ) -> Result<Account, AccountError> {
        let id = Uuid::new_v4();
        let account = Account::new_wallet(id, user_id, name, wallet_address, enabled_chains)?;
        self.repo.save(&account).await?;
        Ok(account)
    }

    /// Retrieve all accounts for a user.
    pub async fn list_accounts(&self, user_id: Uuid) -> Result<Vec<Account>, AccountError> {
        self.repo.find_by_user_id(user_id).await
    }

    /// Retrieve a single account, returning `Err(NotFound)` if absent.
    pub async fn get_account(&self, id: Uuid) -> Result<Account, AccountError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(AccountError::NotFound)
    }

    /// Update holdings after a sync cycle.
    pub async fn update_holdings(
        &self,
        account_id: Uuid,
        holdings: Vec<AccountHolding>,
    ) -> Result<(), AccountError> {
        let mut account = self.get_account(account_id).await?;
        account.sync_holdings(holdings);
        self.repo.save(&account).await
    }

    /// Deactivate an account.
    pub async fn deactivate(&self, account_id: Uuid) -> Result<(), AccountError> {
        let mut account = self.get_account(account_id).await?;
        account.deactivate();
        self.repo.save(&account).await
    }

    /// Delete an account.
    pub async fn delete(&self, account_id: Uuid) -> Result<bool, AccountError> {
        self.repo.delete(account_id).await
    }
}

/// AccountRepository trait — persistence interface for the Account domain.
///
/// Implementations live in `crate::infrastructure::persistence`.
/// Domain code depends only on this trait, never on SeaORM directly.

use async_trait::async_trait;
use uuid::Uuid;

use super::{aggregate::{Account, AccountError}, value_objects::AccountType};

/// Persistence interface for `Account` aggregates.
///
/// All methods are `async` to support both in-memory (test) and database
/// (SeaORM) implementations.
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// Find an account by its primary key.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, AccountError>;

    /// Find all accounts belonging to a user.
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Account>, AccountError>;

    /// Persist a new or modified account.
    async fn save(&self, account: &Account) -> Result<(), AccountError>;

    /// Remove an account by primary key. Returns `true` if it existed.
    async fn delete(&self, id: Uuid) -> Result<bool, AccountError>;

    /// Find all active accounts of a specific type.
    async fn find_active_by_type(
        &self,
        account_type: AccountType,
    ) -> Result<Vec<Account>, AccountError>;
}

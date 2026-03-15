/// Account use cases — application-layer orchestration for account operations.
///
/// Each use case encapsulates a single high-level account operation and
/// delegates to the [`AccountService`] for business logic and repository
/// access. HTTP handlers call use cases; they never touch the repository
/// or domain types directly.
///
/// # Dependency Injection
///
/// `AccountUseCases` is constructed in `main.rs` and shared across
/// all account handlers via Axum's `Extension` extractor:
///
/// ```rust,ignore
/// let use_cases = Arc::new(AccountUseCases::new(account_service));
/// app.layer(Extension(use_cases))
/// ```

use std::sync::Arc;

use uuid::Uuid;

use crate::application::services::account_service::AccountService;
use crate::domains::account::{
    aggregate::{Account, AccountError},
    value_objects::AccountCredentials,
};

// ─── Command types ────────────────────────────────────────────────────────────

/// Input for the "create account" use case.
#[derive(Debug)]
pub struct CreateAccountCommand {
    /// Owning user's identifier.
    pub user_id: Uuid,
    /// Human-readable account name.
    pub name: String,
    /// `"exchange"` or `"wallet"`.
    pub account_type: String,
    /// Required when `account_type == "exchange"`.
    pub exchange_name: Option<String>,
    /// Required when `account_type == "wallet"`.
    pub wallet_address: Option<String>,
    /// EVM chains enabled for wallet accounts.
    pub enabled_chains: Option<Vec<String>>,
    /// API key for exchange accounts (stored encrypted).
    pub api_key: Option<String>,
    /// API secret for exchange accounts (stored encrypted).
    pub api_secret: Option<String>,
    /// Optional passphrase (e.g. OKX requires this).
    pub passphrase: Option<String>,
}

/// Input for the "update account" use case.
#[derive(Debug)]
pub struct UpdateAccountCommand {
    /// Account to update.
    pub id: Uuid,
    /// New name (unchanged when `None`).
    pub name: Option<String>,
    /// New activation state (unchanged when `None`).
    pub is_active: Option<bool>,
    /// Updated API key (unchanged when `None`).
    pub api_key: Option<String>,
    /// Updated API secret (unchanged when `None`).
    pub api_secret: Option<String>,
    /// Updated passphrase (unchanged when `None`).
    pub passphrase: Option<String>,
}

// ─── Use case container ───────────────────────────────────────────────────────

/// Container for all account-related use cases.
///
/// Shared as `Arc<AccountUseCases>` via Axum's `Extension` extractor so that
/// every account handler gets access without re-building the dependency graph
/// on each request.
pub struct AccountUseCases {
    service: Arc<AccountService>,
}

impl AccountUseCases {
    pub fn new(service: Arc<AccountService>) -> Self {
        Self { service }
    }

    /// Return all accounts belonging to `user_id`.
    pub async fn list_accounts(&self, user_id: Uuid) -> Result<Vec<Account>, AccountError> {
        self.service.list_accounts(user_id).await
    }

    /// Return a single account by primary key.
    ///
    /// Returns `Err(AccountError::NotFound)` when absent.
    pub async fn get_account(&self, id: Uuid) -> Result<Account, AccountError> {
        self.service.get_account(id).await
    }

    /// Create a new exchange or wallet account for the given user.
    ///
    /// # Errors
    /// - `AccountError::MissingExchangeName` — exchange account without `exchange_name`
    /// - `AccountError::MissingWalletAddress` — wallet account without `wallet_address`
    /// - `AccountError::InvalidCredentials` — exchange account with empty key/secret
    /// - `AccountError::PersistenceError` — unrecognised `account_type`
    pub async fn create_account(
        &self,
        cmd: CreateAccountCommand,
    ) -> Result<Account, AccountError> {
        match cmd.account_type.as_str() {
            "exchange" => {
                let exchange_name = cmd.exchange_name.ok_or(AccountError::MissingExchangeName)?;
                let api_key = cmd.api_key.unwrap_or_default();
                let api_secret = cmd.api_secret.unwrap_or_default();
                self.service
                    .create_exchange_account(
                        cmd.user_id,
                        cmd.name,
                        exchange_name,
                        api_key,
                        api_secret,
                        cmd.passphrase,
                    )
                    .await
            }
            "wallet" => {
                let wallet_address =
                    cmd.wallet_address.ok_or(AccountError::MissingWalletAddress)?;
                self.service
                    .create_wallet_account(
                        cmd.user_id,
                        cmd.name,
                        wallet_address,
                        cmd.enabled_chains.unwrap_or_default(),
                    )
                    .await
            }
            _ => Err(AccountError::PersistenceError(format!(
                "unknown account_type: {}",
                cmd.account_type
            ))),
        }
    }

    /// Apply a partial update to an existing account.
    ///
    /// Fields set to `None` in the command are left unchanged.
    /// Returns the updated [`Account`].
    ///
    /// # Errors
    /// - `AccountError::NotFound` — account does not exist
    /// - `AccountError::NotAnExchangeAccount` — credential update on wallet
    /// - `AccountError::InvalidCredentials` — empty key/secret after update
    pub async fn update_account(
        &self,
        cmd: UpdateAccountCommand,
    ) -> Result<Account, AccountError> {
        // Build optional credentials only when at least one credential field is provided
        let credentials = match (cmd.api_key, cmd.api_secret) {
            (Some(key), Some(secret)) => {
                Some(AccountCredentials::new(key, secret, cmd.passphrase))
            }
            (Some(key), None) => {
                // Single-field update: fetch existing secret and reuse it
                let existing = self.service.get_account(cmd.id).await?;
                let existing_secret = existing
                    .credentials
                    .as_ref()
                    .map(|c| c.api_secret_encrypted.clone())
                    .unwrap_or_default();
                Some(AccountCredentials::new(key, existing_secret, cmd.passphrase))
            }
            (None, Some(secret)) => {
                // Single-field update: fetch existing key and reuse it
                let existing = self.service.get_account(cmd.id).await?;
                let existing_key = existing
                    .credentials
                    .as_ref()
                    .map(|c| c.api_key_encrypted.clone())
                    .unwrap_or_default();
                Some(AccountCredentials::new(existing_key, secret, cmd.passphrase))
            }
            (None, None) => None,
        };
        self.service
            .update_account(cmd.id, cmd.name, cmd.is_active, credentials)
            .await
    }

    /// Permanently remove an account.
    ///
    /// Returns `true` when the account existed and was deleted, `false` when
    /// no account with that ID was found.
    pub async fn delete_account(&self, id: Uuid) -> Result<bool, AccountError> {
        self.service.delete(id).await
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::account::{
        aggregate::AccountError,
        repository::AccountRepository,
        value_objects::AccountType,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// In-memory repository used by unit tests (no database required).
    struct InMemoryAccountRepository {
        accounts: Mutex<Vec<Account>>,
    }

    impl InMemoryAccountRepository {
        fn new() -> Self {
            Self {
                accounts: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl AccountRepository for InMemoryAccountRepository {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, AccountError> {
            Ok(self.accounts.lock().unwrap().iter().find(|a| a.id == id).cloned())
        }

        async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Account>, AccountError> {
            Ok(self
                .accounts
                .lock()
                .unwrap()
                .iter()
                .filter(|a| a.user_id == user_id)
                .cloned()
                .collect())
        }

        async fn save(&self, account: &Account) -> Result<(), AccountError> {
            let mut accounts = self.accounts.lock().unwrap();
            if let Some(pos) = accounts.iter().position(|a| a.id == account.id) {
                accounts[pos] = account.clone();
            } else {
                accounts.push(account.clone());
            }
            Ok(())
        }

        async fn delete(&self, id: Uuid) -> Result<bool, AccountError> {
            let mut accounts = self.accounts.lock().unwrap();
            let before = accounts.len();
            accounts.retain(|a| a.id != id);
            Ok(accounts.len() < before)
        }

        async fn find_active_by_type(
            &self,
            account_type: AccountType,
        ) -> Result<Vec<Account>, AccountError> {
            Ok(self
                .accounts
                .lock()
                .unwrap()
                .iter()
                .filter(|a| a.account_type == account_type && a.is_active)
                .cloned()
                .collect())
        }
    }

    fn make_use_cases() -> AccountUseCases {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = Arc::new(AccountService::new(repo));
        AccountUseCases::new(service)
    }

    #[tokio::test]
    async fn test_create_exchange_account() {
        let uc = make_use_cases();
        let user_id = Uuid::new_v4();
        let cmd = CreateAccountCommand {
            user_id,
            name: "OKX Main".into(),
            account_type: "exchange".into(),
            exchange_name: Some("OKX".into()),
            wallet_address: None,
            enabled_chains: None,
            api_key: Some("key123".into()),
            api_secret: Some("secret456".into()),
            passphrase: None,
        };
        let account = uc.create_account(cmd).await.unwrap();
        assert_eq!(account.user_id, user_id);
        assert_eq!(account.exchange_name.as_deref(), Some("OKX"));
        assert!(account.is_active);
    }

    #[tokio::test]
    async fn test_create_wallet_account() {
        let uc = make_use_cases();
        let user_id = Uuid::new_v4();
        let cmd = CreateAccountCommand {
            user_id,
            name: "My Wallet".into(),
            account_type: "wallet".into(),
            exchange_name: None,
            wallet_address: Some("0xABC".into()),
            enabled_chains: Some(vec!["ethereum".into()]),
            api_key: None,
            api_secret: None,
            passphrase: None,
        };
        let account = uc.create_account(cmd).await.unwrap();
        assert_eq!(account.wallet_address.as_deref(), Some("0xABC"));
        assert_eq!(account.enabled_chains, vec!["ethereum"]);
    }

    #[tokio::test]
    async fn test_create_account_missing_exchange_name() {
        let uc = make_use_cases();
        let cmd = CreateAccountCommand {
            user_id: Uuid::new_v4(),
            name: "Bad".into(),
            account_type: "exchange".into(),
            exchange_name: None,
            wallet_address: None,
            enabled_chains: None,
            api_key: Some("k".into()),
            api_secret: Some("s".into()),
            passphrase: None,
        };
        assert!(matches!(
            uc.create_account(cmd).await,
            Err(AccountError::MissingExchangeName)
        ));
    }

    #[tokio::test]
    async fn test_create_account_unknown_type() {
        let uc = make_use_cases();
        let cmd = CreateAccountCommand {
            user_id: Uuid::new_v4(),
            name: "Unknown".into(),
            account_type: "unknown".into(),
            exchange_name: None,
            wallet_address: None,
            enabled_chains: None,
            api_key: None,
            api_secret: None,
            passphrase: None,
        };
        assert!(matches!(
            uc.create_account(cmd).await,
            Err(AccountError::PersistenceError(_))
        ));
    }

    #[tokio::test]
    async fn test_list_accounts() {
        let uc = make_use_cases();
        let user_id = Uuid::new_v4();
        for i in 0..3 {
            uc.create_account(CreateAccountCommand {
                user_id,
                name: format!("Wallet {}", i),
                account_type: "wallet".into(),
                exchange_name: None,
                wallet_address: Some(format!("0x{:040x}", i)),
                enabled_chains: None,
                api_key: None,
                api_secret: None,
                passphrase: None,
            })
            .await
            .unwrap();
        }
        let accounts = uc.list_accounts(user_id).await.unwrap();
        assert_eq!(accounts.len(), 3);
    }

    #[tokio::test]
    async fn test_get_account_not_found() {
        let uc = make_use_cases();
        assert!(matches!(
            uc.get_account(Uuid::new_v4()).await,
            Err(AccountError::NotFound)
        ));
    }

    #[tokio::test]
    async fn test_update_account_name() {
        let uc = make_use_cases();
        let user_id = Uuid::new_v4();
        let account = uc
            .create_account(CreateAccountCommand {
                user_id,
                name: "Old Name".into(),
                account_type: "wallet".into(),
                exchange_name: None,
                wallet_address: Some("0xABC".into()),
                enabled_chains: None,
                api_key: None,
                api_secret: None,
                passphrase: None,
            })
            .await
            .unwrap();

        let updated = uc
            .update_account(UpdateAccountCommand {
                id: account.id,
                name: Some("New Name".into()),
                is_active: None,
                api_key: None,
                api_secret: None,
                passphrase: None,
            })
            .await
            .unwrap();
        assert_eq!(updated.name, "New Name");
    }

    #[tokio::test]
    async fn test_delete_account() {
        let uc = make_use_cases();
        let user_id = Uuid::new_v4();
        let account = uc
            .create_account(CreateAccountCommand {
                user_id,
                name: "To Delete".into(),
                account_type: "wallet".into(),
                exchange_name: None,
                wallet_address: Some("0xDEL".into()),
                enabled_chains: None,
                api_key: None,
                api_secret: None,
                passphrase: None,
            })
            .await
            .unwrap();

        let deleted = uc.delete_account(account.id).await.unwrap();
        assert!(deleted);
        // Second delete should return false
        let deleted_again = uc.delete_account(account.id).await.unwrap();
        assert!(!deleted_again);
    }
}

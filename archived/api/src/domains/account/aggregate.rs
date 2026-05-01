/// Account aggregate root
///
/// `Account` is the aggregate root for the Account bounded context. All
/// mutations to account data (holdings, credentials, activation) must go
/// through this type to preserve invariants.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
    entities::{AccountHolding, AccountHoldings},
    value_objects::{AccountCredentials, AccountType},
};

/// The Account aggregate root.
///
/// Encapsulates the complete state of a user account and enforces domain
/// invariants via its methods.
///
/// # Invariants
/// - An exchange account **must** have `exchange_name` and `credentials`.
/// - A wallet account **must** have `wallet_address`.
/// - Holdings are indexed by asset symbol — no duplicates allowed.
/// - Credentials are **never** exposed in serialised API responses.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub account_type: AccountType,
    pub is_active: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Exchange-specific fields
    pub exchange_name: Option<String>,
    pub credentials: Option<AccountCredentials>,

    // Wallet-specific fields
    pub wallet_address: Option<String>,
    pub enabled_chains: Vec<String>,

    // Holdings (current state, upserted on each sync)
    holdings: AccountHoldings,
}

impl Account {
    /// Create a new exchange account.
    ///
    /// # Errors
    /// Returns `Err` if `exchange_name` is empty or credentials are invalid.
    pub fn new_exchange(
        id: Uuid,
        user_id: Uuid,
        name: impl Into<String>,
        exchange_name: impl Into<String>,
        credentials: AccountCredentials,
    ) -> Result<Self, AccountError> {
        let exchange_name = exchange_name.into();
        if exchange_name.is_empty() {
            return Err(AccountError::MissingExchangeName);
        }
        if !credentials.is_valid() {
            return Err(AccountError::InvalidCredentials);
        }
        let now = Utc::now();
        Ok(Self {
            id,
            user_id,
            name: name.into(),
            account_type: AccountType::Exchange,
            is_active: true,
            last_synced_at: None,
            created_at: now,
            updated_at: now,
            exchange_name: Some(exchange_name),
            credentials: Some(credentials),
            wallet_address: None,
            enabled_chains: Vec::new(),
            holdings: AccountHoldings::default(),
        })
    }

    /// Create a new wallet account.
    ///
    /// # Errors
    /// Returns `Err` if `wallet_address` is empty.
    pub fn new_wallet(
        id: Uuid,
        user_id: Uuid,
        name: impl Into<String>,
        wallet_address: impl Into<String>,
        enabled_chains: Vec<String>,
    ) -> Result<Self, AccountError> {
        let wallet_address = wallet_address.into();
        if wallet_address.is_empty() {
            return Err(AccountError::MissingWalletAddress);
        }
        let now = Utc::now();
        Ok(Self {
            id,
            user_id,
            name: name.into(),
            account_type: AccountType::Wallet,
            is_active: true,
            last_synced_at: None,
            created_at: now,
            updated_at: now,
            exchange_name: None,
            credentials: None,
            wallet_address: Some(wallet_address),
            enabled_chains,
            holdings: AccountHoldings::default(),
        })
    }

    /// Reconstruct an `Account` from persisted state (used by repository implementations).
    #[allow(clippy::too_many_arguments)]
    pub fn from_persistence(
        id: Uuid,
        user_id: Uuid,
        name: String,
        account_type: AccountType,
        is_active: bool,
        last_synced_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        exchange_name: Option<String>,
        credentials: Option<AccountCredentials>,
        wallet_address: Option<String>,
        enabled_chains: Vec<String>,
        holdings: AccountHoldings,
    ) -> Self {
        Self {
            id,
            user_id,
            name,
            account_type,
            is_active,
            last_synced_at,
            created_at,
            updated_at,
            exchange_name,
            credentials,
            wallet_address,
            enabled_chains,
            holdings,
        }
    }

    // ─── Query methods ──────────────────────────────────────────────────────────

    pub fn holdings(&self) -> &AccountHoldings {
        &self.holdings
    }

    pub fn holding_for(&self, asset: &str) -> Option<&AccountHolding> {
        self.holdings.find(asset)
    }

    // ─── Command methods ─────────────────────────────────────────────────────────

    /// Replace all holdings in a single atomic operation.
    ///
    /// Used by the account sync job to install fresh data from the connector.
    /// Sets `last_synced_at` to now.
    pub fn sync_holdings(&mut self, new_holdings: Vec<AccountHolding>) {
        self.holdings.replace_all(new_holdings);
        self.last_synced_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Add or update a single holding.
    pub fn add_holding(&mut self, holding: AccountHolding) {
        self.holdings.upsert(holding);
        self.updated_at = Utc::now();
    }

    /// Remove a holding by asset symbol. Returns `true` if removed.
    pub fn remove_holding(&mut self, asset: &str) -> bool {
        let removed = self.holdings.remove(asset);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Activate the account.
    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Utc::now();
    }

    /// Deactivate the account (pauses sync).
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }

    /// Update credentials for an exchange account.
    ///
    /// # Errors
    /// Returns `Err` if this is not an exchange account or credentials are invalid.
    pub fn update_credentials(
        &mut self,
        credentials: AccountCredentials,
    ) -> Result<(), AccountError> {
        if !self.account_type.is_exchange() {
            return Err(AccountError::NotAnExchangeAccount);
        }
        if !credentials.is_valid() {
            return Err(AccountError::InvalidCredentials);
        }
        self.credentials = Some(credentials);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Rename the account.
    pub fn rename(&mut self, new_name: impl Into<String>) {
        self.name = new_name.into();
        self.updated_at = Utc::now();
    }

    /// Validate that the account satisfies all type-specific invariants.
    pub fn validate(&self) -> Result<(), AccountError> {
        match &self.account_type {
            AccountType::Exchange => {
                if self.exchange_name.as_deref().map_or(true, str::is_empty) {
                    return Err(AccountError::MissingExchangeName);
                }
                match &self.credentials {
                    None => return Err(AccountError::InvalidCredentials),
                    Some(c) if !c.is_valid() => return Err(AccountError::InvalidCredentials),
                    _ => {}
                }
            }
            AccountType::Wallet => {
                if self.wallet_address.as_deref().map_or(true, str::is_empty) {
                    return Err(AccountError::MissingWalletAddress);
                }
            }
        }
        Ok(())
    }
}

/// Domain errors for the Account aggregate.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AccountError {
    #[error("exchange_name is required for exchange accounts")]
    MissingExchangeName,
    #[error("wallet_address is required for wallet accounts")]
    MissingWalletAddress,
    #[error("credentials are invalid or missing (api_key and api_secret required)")]
    InvalidCredentials,
    #[error("operation is only valid for exchange accounts")]
    NotAnExchangeAccount,
    #[error("account not found")]
    NotFound,
    #[error("persistence error: {0}")]
    PersistenceError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_creds() -> AccountCredentials {
        AccountCredentials::new("key".into(), "secret".into(), None)
    }

    #[test]
    fn test_create_exchange_account() {
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let account = Account::new_exchange(id, user_id, "My OKX", "OKX", make_creds()).unwrap();
        assert_eq!(account.account_type, AccountType::Exchange);
        assert!(account.is_active);
        assert!(account.last_synced_at.is_none());
        assert_eq!(account.exchange_name.as_deref(), Some("OKX"));
    }

    #[test]
    fn test_create_exchange_account_missing_name() {
        let err = Account::new_exchange(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "acc",
            "",
            make_creds(),
        )
        .unwrap_err();
        assert_eq!(err, AccountError::MissingExchangeName);
    }

    #[test]
    fn test_create_wallet_account() {
        let account = Account::new_wallet(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "My Wallet",
            "0xABC",
            vec!["ethereum".into()],
        )
        .unwrap();
        assert_eq!(account.account_type, AccountType::Wallet);
        assert_eq!(account.wallet_address.as_deref(), Some("0xABC"));
        assert_eq!(account.enabled_chains, vec!["ethereum"]);
    }

    #[test]
    fn test_create_wallet_missing_address() {
        let err = Account::new_wallet(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "acc",
            "",
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, AccountError::MissingWalletAddress);
    }

    #[test]
    fn test_sync_holdings() {
        let mut account =
            Account::new_exchange(Uuid::new_v4(), Uuid::new_v4(), "acc", "OKX", make_creds())
                .unwrap();

        let holdings = vec![
            AccountHolding::new("BTC", "1.0"),
            AccountHolding::new("ETH", "5.0"),
        ];
        account.sync_holdings(holdings);

        assert_eq!(account.holdings().len(), 2);
        assert!(account.last_synced_at.is_some());
    }

    #[test]
    fn test_deactivate_and_activate() {
        let mut account =
            Account::new_exchange(Uuid::new_v4(), Uuid::new_v4(), "acc", "OKX", make_creds())
                .unwrap();
        account.deactivate();
        assert!(!account.is_active);
        account.activate();
        assert!(account.is_active);
    }
}

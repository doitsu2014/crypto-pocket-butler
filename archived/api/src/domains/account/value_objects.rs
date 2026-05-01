/// Account value objects: AccountType and AccountCredentials
///
/// Value objects are immutable and defined by their attributes (not identity).
/// They encapsulate business rules for account classification and credential management.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Discriminates between exchange-connected accounts and self-custody wallets.
///
/// # Invariants
/// - An `Exchange` account requires `exchange_name` and `AccountCredentials`.
/// - A `Wallet` account requires a `wallet_address` and optional `enabled_chains`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    /// Centralised exchange account authenticated via API key / secret.
    Exchange,
    /// Self-custody wallet identified by a public address.
    Wallet,
}

impl AccountType {
    /// Parse from the database string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "exchange" => Some(Self::Exchange),
            "wallet" => Some(Self::Wallet),
            _ => None,
        }
    }

    /// Convert to the database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exchange => "exchange",
            Self::Wallet => "wallet",
        }
    }

    pub fn is_exchange(&self) -> bool {
        matches!(self, Self::Exchange)
    }

    pub fn is_wallet(&self) -> bool {
        matches!(self, Self::Wallet)
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Encrypted API credentials for an exchange account.
///
/// # Security
/// All credential fields are encrypted before persistence and are **never** returned
/// in API responses (fields are marked `#[serde(skip_serializing)]`).
///
/// # Invariants
/// - `api_key_encrypted` must be non-empty.
/// - `api_secret_encrypted` must be non-empty.
/// - `passphrase_encrypted` is optional (some exchanges do not require it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCredentials {
    /// Encrypted API key.
    #[serde(skip_serializing)]
    pub api_key_encrypted: String,
    /// Encrypted API secret.
    #[serde(skip_serializing)]
    pub api_secret_encrypted: String,
    /// Encrypted passphrase (optional, e.g. OKX requires this).
    #[serde(skip_serializing)]
    pub passphrase_encrypted: Option<String>,
}

impl AccountCredentials {
    pub fn new(
        api_key_encrypted: String,
        api_secret_encrypted: String,
        passphrase_encrypted: Option<String>,
    ) -> Self {
        Self {
            api_key_encrypted,
            api_secret_encrypted,
            passphrase_encrypted,
        }
    }

    /// Returns `true` if all required fields are present and non-empty.
    pub fn is_valid(&self) -> bool {
        !self.api_key_encrypted.is_empty() && !self.api_secret_encrypted.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_type_from_str() {
        assert_eq!(AccountType::from_str("exchange"), Some(AccountType::Exchange));
        assert_eq!(AccountType::from_str("wallet"), Some(AccountType::Wallet));
        assert_eq!(AccountType::from_str("EXCHANGE"), Some(AccountType::Exchange));
        assert_eq!(AccountType::from_str("unknown"), None);
    }

    #[test]
    fn test_account_type_as_str() {
        assert_eq!(AccountType::Exchange.as_str(), "exchange");
        assert_eq!(AccountType::Wallet.as_str(), "wallet");
    }

    #[test]
    fn test_account_type_predicates() {
        assert!(AccountType::Exchange.is_exchange());
        assert!(!AccountType::Exchange.is_wallet());
        assert!(AccountType::Wallet.is_wallet());
        assert!(!AccountType::Wallet.is_exchange());
    }

    #[test]
    fn test_credentials_valid() {
        let creds = AccountCredentials::new("key".into(), "secret".into(), None);
        assert!(creds.is_valid());
    }

    #[test]
    fn test_credentials_empty_key_invalid() {
        let creds = AccountCredentials::new(String::new(), "secret".into(), None);
        assert!(!creds.is_valid());
    }

    #[test]
    fn test_credentials_not_serialized() {
        let creds = AccountCredentials::new("key".into(), "secret".into(), Some("pass".into()));
        let json = serde_json::to_string(&creds).unwrap();
        assert!(!json.contains("key"));
        assert!(!json.contains("secret"));
        assert!(!json.contains("pass"));
    }
}

//! Wallet addresses and WalletConnect connections. A public address is stored;
//! a signing key or seed phrase never is (BR3.2, BR3.3).

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{AccountId, WalletConnectionId};

/// The longest opaque WalletConnect reference accepted.
pub const MAX_SESSION_REF_CHARS: usize = 200;

/// Validates an address against its chain's format (BR3.2).
///
/// # Errors
/// Returns [`DomainError::Invalid`] when the chain is unknown or the address does
/// not match that chain's shape. The address itself is never echoed.
pub fn validate_address(chain: &str, address: &str) -> Result<(), DomainError> {
    let value = address.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "address" });
    }
    if chain == "solana" {
        let valid = (32..=44).contains(&value.len())
            && value.chars().all(|character| {
                character.is_ascii_alphanumeric() && !matches!(character, '0' | 'O' | 'I' | 'l')
            });
        if valid {
            return Ok(());
        }
        return Err(DomainError::Invalid {
            field: "address",
            reason: "is not a Solana address".to_string(),
        });
    }
    if chain.starts_with("evm:") {
        let body = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
            .unwrap_or("");
        let valid = body.len() == 40 && body.chars().all(|character| character.is_ascii_hexdigit());
        if valid {
            return Ok(());
        }
        return Err(DomainError::Invalid {
            field: "address",
            reason: "is not an EVM address".to_string(),
        });
    }
    Err(DomainError::Invalid {
        field: "chain",
        reason: "is not a supported chain".to_string(),
    })
}

/// A public wallet address or a WalletConnect read-only connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletConnection {
    id: WalletConnectionId,
    account_id: AccountId,
    address: String,
    observed_chain: String,
    session_ref: Option<String>,
    connected_at: DateTime<Utc>,
}

impl WalletConnection {
    /// Creates a wallet address connection (WF2).
    ///
    /// # Errors
    /// Returns the address refusal.
    pub fn new_address(
        id: WalletConnectionId,
        account_id: AccountId,
        chain: &str,
        address: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        validate_address(chain, address)?;
        Ok(Self {
            id,
            account_id,
            address: address.trim().to_string(),
            observed_chain: chain.trim().to_ascii_lowercase(),
            session_ref: None,
            connected_at: now,
        })
    }

    /// Creates a WalletConnect connection with an opaque session reference
    /// (WF3). No signing key is stored.
    ///
    /// # Errors
    /// Returns the address refusal or [`DomainError::TooLarge`] for a long
    /// session reference.
    pub fn walletconnect(
        id: WalletConnectionId,
        account_id: AccountId,
        chain: &str,
        address: &str,
        session_ref: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        validate_address(chain, address)?;
        let reference = session_ref.trim();
        if reference.is_empty() {
            return Err(DomainError::Required {
                field: "sessionRef",
            });
        }
        if reference.chars().count() > MAX_SESSION_REF_CHARS {
            return Err(DomainError::TooLarge {
                field: "sessionRef",
                limit: "200 characters",
            });
        }
        Ok(Self {
            id,
            account_id,
            address: address.trim().to_string(),
            observed_chain: chain.trim().to_ascii_lowercase(),
            session_ref: Some(reference.to_string()),
            connected_at: now,
        })
    }

    /// Rehydrates a stored connection.
    #[must_use]
    pub fn from_stored(
        id: WalletConnectionId,
        account_id: AccountId,
        address: String,
        observed_chain: String,
        session_ref: Option<String>,
        connected_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            account_id,
            address,
            observed_chain,
            session_ref,
            connected_at,
        }
    }

    /// The connection identifier.
    #[must_use]
    pub fn id(&self) -> WalletConnectionId {
        self.id
    }

    /// The owning account.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// The public address.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// The chain the address was validated against.
    #[must_use]
    pub fn observed_chain(&self) -> &str {
        &self.observed_chain
    }

    /// The opaque WalletConnect session reference, when one exists.
    #[must_use]
    pub fn session_ref(&self) -> Option<&str> {
        self.session_ref.as_deref()
    }

    /// When the connection was established.
    #[must_use]
    pub fn connected_at(&self) -> DateTime<Utc> {
        self.connected_at
    }
}

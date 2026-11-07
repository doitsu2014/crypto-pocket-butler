//! The Account aggregate and its kind, venue, and status value types (BR1.1–BR1.4).

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{AccountId, UserId};

/// The longest account label the service accepts.
pub const MAX_LABEL_CHARS: usize = 64;

/// Where a read-only holding source comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    /// An exchange read-only credential.
    Exchange,
    /// A public wallet address.
    Wallet,
    /// A WalletConnect read-only session.
    WalletConnect,
    /// A CSV import surface.
    Csv,
}

impl AccountKind {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exchange => "exchange",
            Self::Wallet => "wallet",
            Self::WalletConnect => "walletconnect",
            Self::Csv => "csv",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown kind.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "exchange" => Ok(Self::Exchange),
            "wallet" => Ok(Self::Wallet),
            "walletconnect" => Ok(Self::WalletConnect),
            "csv" => Ok(Self::Csv),
            _ => Err(DomainError::Invalid {
                field: "kind",
                reason: "must be one of exchange, wallet, walletconnect, csv".to_string(),
            }),
        }
    }
}

/// The account lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    /// The account is ready and not syncing.
    Active,
    /// A sync run is in progress.
    Syncing,
    /// The last sync failed; the previous holdings are kept.
    Error,
    /// The credential was revoked; the account is out of the portfolio.
    Disconnected,
}

impl AccountStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Syncing => "syncing",
            Self::Error => "error",
            Self::Disconnected => "disconnected",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "syncing" => Ok(Self::Syncing),
            "error" => Ok(Self::Error),
            "disconnected" => Ok(Self::Disconnected),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be one of active, syncing, error, disconnected".to_string(),
            }),
        }
    }
}

/// The source venue or chain. Exchange venues are `binance` and `okx`; wallet
/// venues are `evm:<chainId>` or `solana`; a CSV venue is `csv:<label>` (BR3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Venue(String);

impl Venue {
    /// Validates a venue for an account kind.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the venue does not fit the kind, and
    /// [`DomainError::Required`] when it is blank.
    pub fn parse_for(kind: AccountKind, raw: &str) -> Result<Self, DomainError> {
        let value = raw.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Err(DomainError::Required { field: "venue" });
        }
        let valid = match kind {
            AccountKind::Exchange => matches!(value.as_str(), "binance" | "okx"),
            AccountKind::Wallet | AccountKind::WalletConnect => is_chain_venue(&value),
            AccountKind::Csv => value
                .strip_prefix("csv:")
                .is_some_and(|label| !label.is_empty() && label.chars().count() <= MAX_LABEL_CHARS),
        };
        if valid {
            Ok(Self(value))
        } else {
            Err(DomainError::Invalid {
                field: "venue",
                reason: match kind {
                    AccountKind::Exchange => "must be binance or okx".to_string(),
                    AccountKind::Wallet | AccountKind::WalletConnect => {
                        "must be evm:<chainId> or solana".to_string()
                    }
                    AccountKind::Csv => "must be csv:<label>".to_string(),
                },
            })
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the exchange name for an exchange venue.
    #[must_use]
    pub fn exchange(&self) -> Option<&str> {
        matches!(self.0.as_str(), "binance" | "okx").then_some(self.0.as_str())
    }

    /// Returns the chain identifier for a wallet venue.
    #[must_use]
    pub fn chain(&self) -> Option<&str> {
        is_chain_venue(&self.0).then_some(self.0.as_str())
    }
}

fn is_chain_venue(value: &str) -> bool {
    if value == "solana" {
        return true;
    }
    let Some(chain_id) = value.strip_prefix("evm:") else {
        return false;
    };
    !chain_id.is_empty()
        && chain_id.len() <= 20
        && chain_id.chars().all(|character| character.is_ascii_digit())
        && chain_id.parse::<u64>().is_ok()
}

/// Trims and validates an account label. The original casing is kept for display.
///
/// # Errors
/// Returns [`DomainError::Required`] when blank and
/// [`DomainError::TooLarge`] when longer than [`MAX_LABEL_CHARS`].
pub fn validate_label(raw: &str) -> Result<String, DomainError> {
    let label = raw.trim();
    if label.is_empty() {
        return Err(DomainError::Required { field: "label" });
    }
    if label.chars().count() > MAX_LABEL_CHARS {
        return Err(DomainError::TooLarge {
            field: "label",
            limit: "64 characters",
        });
    }
    Ok(label.to_string())
}

/// The case-folded form used by the uniqueness index (BR1.2).
#[must_use]
pub fn normalise_label(label: &str) -> String {
    label.trim().to_lowercase()
}

/// One read-only source of holdings for a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    user_id: UserId,
    kind: AccountKind,
    venue: Venue,
    label: String,
    status: AccountStatus,
    last_synced_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    anonymised_at: Option<DateTime<Utc>>,
}

impl Account {
    /// Opens a new active account. The caller has already verified any credential.
    ///
    /// # Errors
    /// Returns the label or venue refusal.
    pub fn open(
        id: AccountId,
        user_id: UserId,
        kind: AccountKind,
        venue: &str,
        label: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let label = validate_label(label)?;
        let venue = Venue::parse_for(kind, venue)?;
        Ok(Self {
            id,
            user_id,
            kind,
            venue,
            label,
            status: AccountStatus::Active,
            last_synced_at: None,
            created_at: now,
            updated_at: now,
            anonymised_at: None,
        })
    }

    /// Rehydrates a stored account.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: AccountId,
        user_id: UserId,
        kind: AccountKind,
        venue: Venue,
        label: String,
        status: AccountStatus,
        last_synced_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        anonymised_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            kind,
            venue,
            label,
            status,
            last_synced_at,
            created_at,
            updated_at,
            anonymised_at,
        }
    }

    /// The account identifier.
    #[must_use]
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// The owning user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The source kind.
    #[must_use]
    pub fn kind(&self) -> AccountKind {
        self.kind
    }

    /// The venue or chain.
    #[must_use]
    pub fn venue(&self) -> &Venue {
        &self.venue
    }

    /// The display label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> AccountStatus {
        self.status
    }

    /// The last successful sync, when one has happened.
    #[must_use]
    pub fn last_synced_at(&self) -> Option<DateTime<Utc>> {
        self.last_synced_at
    }

    /// Creation time.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Last change time.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// When the account was anonymised for privacy deletion.
    #[must_use]
    pub fn anonymised_at(&self) -> Option<DateTime<Utc>> {
        self.anonymised_at
    }

    /// Whether the account is a CSV history-only account.
    #[must_use]
    pub fn is_csv(&self) -> bool {
        self.kind == AccountKind::Csv
    }

    /// Whether the account may take part in a holdings sync.
    #[must_use]
    pub fn is_syncable(&self) -> bool {
        self.kind != AccountKind::Csv && self.status != AccountStatus::Disconnected
    }

    /// Changes the label (BR1.2 is enforced by the repository's unique index).
    ///
    /// # Errors
    /// Returns the label refusal.
    pub fn relabel(&mut self, label: &str, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.label = validate_label(label)?;
        self.updated_at = now;
        Ok(())
    }

    /// Moves an active or failed account into syncing (BR4.3). A disconnected
    /// account refuses until a new credential verifies (BR4.5).
    ///
    /// # Errors
    /// Returns [`DomainError::Denied`] for a disconnected account.
    pub fn begin_sync(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status == AccountStatus::Disconnected {
            return Err(DomainError::Denied {
                reason: "the account is disconnected until a new credential verifies".to_string(),
            });
        }
        self.status = AccountStatus::Syncing;
        self.updated_at = now;
        Ok(())
    }

    /// Records a successful sync and its time (BR4.3).
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] outside the syncing state.
    pub fn finish_sync_success(&mut self, at: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != AccountStatus::Syncing {
            return Err(DomainError::InvalidTransition {
                entity: "account",
                from: self.status.as_str(),
                to: AccountStatus::Active.as_str(),
            });
        }
        self.status = AccountStatus::Active;
        self.last_synced_at = Some(at);
        self.updated_at = at;
        Ok(())
    }

    /// Records a failed sync while keeping the previous holdings (BR4.4).
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] outside the syncing state.
    pub fn finish_sync_failure(&mut self, at: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != AccountStatus::Syncing {
            return Err(DomainError::InvalidTransition {
                entity: "account",
                from: self.status.as_str(),
                to: AccountStatus::Error.as_str(),
            });
        }
        self.status = AccountStatus::Error;
        self.updated_at = at;
        Ok(())
    }

    /// Marks the account disconnected after a credential revocation (BR2.5).
    pub fn disconnect(&mut self, at: DateTime<Utc>) {
        self.status = AccountStatus::Disconnected;
        self.updated_at = at;
    }

    /// Returns a disconnected account to active after a new credential verifies
    /// (BR4.5).
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] from a non-disconnected state.
    pub fn reactivate(&mut self, at: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != AccountStatus::Disconnected {
            return Err(DomainError::InvalidTransition {
                entity: "account",
                from: self.status.as_str(),
                to: AccountStatus::Active.as_str(),
            });
        }
        self.status = AccountStatus::Active;
        self.updated_at = at;
        Ok(())
    }

    /// Anonymises the personal label for privacy deletion while keeping the
    /// financial-retention rows (BR8.2).
    pub fn anonymise(&mut self, at: DateTime<Utc>) {
        self.label = format!("anonymised-{}", self.id);
        self.anonymised_at = Some(at);
        self.updated_at = at;
    }
}

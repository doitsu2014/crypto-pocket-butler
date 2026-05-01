/// Portfolio aggregate root

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
    entities::PortfolioAccount,
    value_objects::{Guardrails, TargetAllocation},
};

/// The Portfolio aggregate root.
///
/// # Invariants
/// - `name` is unique per user (enforced at the persistence layer).
/// - The first portfolio created for a user is automatically the default.
/// - An account can only be added to the same portfolio once.
/// - Target allocation weights must sum to 1.0 when set.
#[derive(Debug, Clone)]
pub struct Portfolio {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub target_allocation: TargetAllocation,
    pub guardrails: Option<Guardrails>,
    pub last_constructed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    accounts: Vec<PortfolioAccount>,
}

impl Portfolio {
    /// Create a new portfolio.
    ///
    /// `is_default` should be `true` only for the first portfolio of a user.
    pub fn new(
        id: Uuid,
        user_id: Uuid,
        name: impl Into<String>,
        description: Option<String>,
        is_default: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            name: name.into(),
            description,
            is_default,
            target_allocation: TargetAllocation::default(),
            guardrails: None,
            last_constructed_at: None,
            created_at: now,
            updated_at: now,
            accounts: Vec::new(),
        }
    }

    /// Reconstruct from persisted state.
    #[allow(clippy::too_many_arguments)]
    pub fn from_persistence(
        id: Uuid,
        user_id: Uuid,
        name: String,
        description: Option<String>,
        is_default: bool,
        target_allocation: TargetAllocation,
        guardrails: Option<Guardrails>,
        last_constructed_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        accounts: Vec<PortfolioAccount>,
    ) -> Self {
        Self {
            id,
            user_id,
            name,
            description,
            is_default,
            target_allocation,
            guardrails,
            last_constructed_at,
            created_at,
            updated_at,
            accounts,
        }
    }

    // ─── Query methods ──────────────────────────────────────────────────────────

    pub fn accounts(&self) -> &[PortfolioAccount] {
        &self.accounts
    }

    pub fn account_ids(&self) -> Vec<Uuid> {
        self.accounts.iter().map(|a| a.account_id).collect()
    }

    pub fn contains_account(&self, account_id: Uuid) -> bool {
        self.accounts.iter().any(|a| a.account_id == account_id)
    }

    // ─── Command methods ─────────────────────────────────────────────────────────

    /// Add an account to this portfolio.
    ///
    /// # Errors
    /// Returns `Err(PortfolioError::AccountAlreadyAdded)` if already present.
    pub fn add_account(
        &mut self,
        account_id: Uuid,
    ) -> Result<(), PortfolioError> {
        if self.contains_account(account_id) {
            return Err(PortfolioError::AccountAlreadyAdded(account_id));
        }
        self.accounts.push(PortfolioAccount::new(self.id, account_id));
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Remove an account from this portfolio.
    ///
    /// Returns `true` if the account was present and removed.
    pub fn remove_account(&mut self, account_id: Uuid) -> bool {
        let before = self.accounts.len();
        self.accounts.retain(|a| a.account_id != account_id);
        let removed = self.accounts.len() < before;
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Mark this portfolio as the default.
    pub fn set_as_default(&mut self) {
        self.is_default = true;
        self.updated_at = Utc::now();
    }

    /// Set the target allocation. Validates weights sum to 1.0.
    pub fn set_target_allocation(
        &mut self,
        allocation: TargetAllocation,
    ) -> Result<(), PortfolioError> {
        allocation
            .validate()
            .map_err(|e| PortfolioError::InvalidTargetAllocation(e.to_string()))?;
        self.target_allocation = allocation;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Set guardrails.
    pub fn set_guardrails(&mut self, guardrails: Guardrails) {
        self.guardrails = Some(guardrails);
        self.updated_at = Utc::now();
    }

    /// Update `last_constructed_at` after an allocation has been computed.
    pub fn mark_constructed(&mut self) {
        self.last_constructed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

/// Domain errors for the Portfolio aggregate.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PortfolioError {
    #[error("account {0} is already in this portfolio")]
    AccountAlreadyAdded(Uuid),
    #[error("target allocation is invalid: {0}")]
    InvalidTargetAllocation(String),
    #[error("portfolio not found")]
    NotFound,
    #[error("a portfolio with this name already exists")]
    DuplicateName,
    #[error("persistence error: {0}")]
    PersistenceError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_portfolio_is_default() {
        let p = Portfolio::new(Uuid::new_v4(), Uuid::new_v4(), "My Portfolio", None, true);
        assert!(p.is_default);
        assert!(p.accounts().is_empty());
    }

    #[test]
    fn test_add_account() {
        let mut p = Portfolio::new(Uuid::new_v4(), Uuid::new_v4(), "P", None, true);
        let acc_id = Uuid::new_v4();
        assert!(p.add_account(acc_id).is_ok());
        assert!(p.contains_account(acc_id));
    }

    #[test]
    fn test_add_account_twice_errors() {
        let mut p = Portfolio::new(Uuid::new_v4(), Uuid::new_v4(), "P", None, true);
        let acc_id = Uuid::new_v4();
        p.add_account(acc_id).unwrap();
        let err = p.add_account(acc_id).unwrap_err();
        assert_eq!(err, PortfolioError::AccountAlreadyAdded(acc_id));
    }

    #[test]
    fn test_remove_account() {
        let mut p = Portfolio::new(Uuid::new_v4(), Uuid::new_v4(), "P", None, true);
        let acc_id = Uuid::new_v4();
        p.add_account(acc_id).unwrap();
        assert!(p.remove_account(acc_id));
        assert!(!p.contains_account(acc_id));
        assert!(!p.remove_account(acc_id)); // already gone
    }
}

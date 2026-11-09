//! Account membership (BR1.3): an account belongs to at most one portfolio.

use chrono::{DateTime, Utc};

use crate::ids::{AccountId, PortfolioAccountId, PortfolioId};

/// One account assigned to exactly one portfolio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioAccount {
    id: PortfolioAccountId,
    portfolio_id: PortfolioId,
    account_id: AccountId,
    added_at: DateTime<Utc>,
}

impl PortfolioAccount {
    /// Creates a membership row.
    #[must_use]
    pub fn new(
        id: PortfolioAccountId,
        portfolio_id: PortfolioId,
        account_id: AccountId,
        added_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            portfolio_id,
            account_id,
            added_at,
        }
    }

    /// Rehydrates a stored membership.
    #[must_use]
    pub fn from_stored(
        id: PortfolioAccountId,
        portfolio_id: PortfolioId,
        account_id: AccountId,
        added_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            portfolio_id,
            account_id,
            added_at,
        }
    }

    /// The membership identifier.
    #[must_use]
    pub fn id(&self) -> PortfolioAccountId {
        self.id
    }

    /// The portfolio the account belongs to.
    #[must_use]
    pub fn portfolio_id(&self) -> PortfolioId {
        self.portfolio_id
    }

    /// The account.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// When the account joined the portfolio.
    #[must_use]
    pub fn added_at(&self) -> DateTime<Utc> {
        self.added_at
    }
}

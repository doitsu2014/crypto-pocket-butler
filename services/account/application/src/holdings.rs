//! WF6: the C5 holdings and transactions reads (BR6.1–BR6.3).

use std::sync::Arc;

use chrono::{DateTime, Utc};

use account_domain::{AccountId, HoldingSnapshot, ImportedTransaction, Operation, UserId};

use crate::common::{AppError, ServiceCaller};
use crate::ports::{AccountRepository, HoldingRepository, ImportRepository};

/// One account's latest holdings and the time they were observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountHoldings {
    /// The account.
    pub account_id: AccountId,
    /// The last successful sync, when one has happened.
    pub last_synced_at: Option<DateTime<Utc>>,
    /// The latest snapshot, empty when no sync has completed.
    pub holdings: Vec<HoldingSnapshot>,
}

/// Serves the C5 reads Portfolio and Ledger call.
pub struct HoldingService {
    accounts: Arc<dyn AccountRepository>,
    holdings: Arc<dyn HoldingRepository>,
    imports: Arc<dyn ImportRepository>,
}

impl HoldingService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        holdings: Arc<dyn HoldingRepository>,
        imports: Arc<dyn ImportRepository>,
    ) -> Self {
        Self {
            accounts,
            holdings,
            imports,
        }
    }

    /// Returns each of the user's accounts with its latest snapshot.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the holdings
    /// operation, and a repository refusal when the read fails.
    pub async fn for_user(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
    ) -> Result<Vec<AccountHoldings>, AppError> {
        caller.require_operation(Operation::HoldingsRead)?;
        let accounts = self
            .accounts
            .find_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;
        let mut result = Vec::with_capacity(accounts.len());
        for account in accounts {
            let holdings = self
                .holdings
                .latest_for_account(account.id())
                .await
                .map_err(AppError::from_repository)?;
            result.push(AccountHoldings {
                account_id: account.id(),
                last_synced_at: account.last_synced_at(),
                holdings,
            });
        }
        Ok(result)
    }

    /// Returns the account's imported transactions since `since`.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the transactions
    /// operation, and a repository refusal when the read fails.
    pub async fn transactions(
        &self,
        caller: &ServiceCaller,
        account_id: AccountId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ImportedTransaction>, AppError> {
        caller.require_operation(Operation::TransactionsRead)?;
        self.imports
            .transactions_for_account(account_id, since)
            .await
            .map_err(AppError::from_repository)
    }
}

/// Portfolio use cases — application-layer orchestration for portfolio operations.
///
/// Each use case encapsulates a single high-level portfolio operation and
/// delegates to [`PortfolioService`] for business logic and repository access.
/// HTTP handlers should call these use cases; they must not touch the
/// repository or domain types directly.

use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::portfolio_service::PortfolioService;
use crate::domains::portfolio::aggregate::{Portfolio, PortfolioError};

// ─── Command types ────────────────────────────────────────────────────────────

/// Input for the "create portfolio" use case.
#[derive(Debug)]
pub struct CreatePortfolioCommand {
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

/// Input for the "add account to portfolio" use case.
#[derive(Debug)]
pub struct AddAccountCommand {
    pub portfolio_id: Uuid,
    pub account_id: Uuid,
}

/// Input for the "remove account from portfolio" use case.
#[derive(Debug)]
pub struct RemoveAccountCommand {
    pub portfolio_id: Uuid,
    pub account_id: Uuid,
}

// ─── Use case container ───────────────────────────────────────────────────────

/// Container for all portfolio-related use cases.
///
/// Shared as `Arc<PortfolioUseCases>` via Axum's `Extension` extractor.
pub struct PortfolioUseCases {
    service: Arc<PortfolioService>,
}

impl PortfolioUseCases {
    pub fn new(service: Arc<PortfolioService>) -> Self {
        Self { service }
    }

    /// Return all portfolios belonging to `user_id`.
    pub async fn list_portfolios(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Portfolio>, PortfolioError> {
        self.service.list_portfolios(user_id).await
    }

    /// Return a single portfolio by primary key.
    pub async fn get_portfolio(&self, id: Uuid) -> Result<Portfolio, PortfolioError> {
        self.service.get_portfolio(id).await
    }

    /// Create a new portfolio for the given user.
    ///
    /// The first portfolio created for a user is automatically set as default.
    pub async fn create_portfolio(
        &self,
        cmd: CreatePortfolioCommand,
    ) -> Result<Portfolio, PortfolioError> {
        self.service
            .create_portfolio(cmd.user_id, cmd.name, cmd.description)
            .await
    }

    /// Add an account to a portfolio.
    ///
    /// # Errors
    /// - `PortfolioError::NotFound` — portfolio does not exist
    /// - `PortfolioError::AccountAlreadyAdded` — account already in portfolio
    pub async fn add_account(&self, cmd: AddAccountCommand) -> Result<(), PortfolioError> {
        self.service
            .add_account(cmd.portfolio_id, cmd.account_id)
            .await
    }

    /// Remove an account from a portfolio.
    ///
    /// Returns `true` if the account was present and removed.
    pub async fn remove_account(
        &self,
        cmd: RemoveAccountCommand,
    ) -> Result<bool, PortfolioError> {
        self.service
            .remove_account(cmd.portfolio_id, cmd.account_id)
            .await
    }

    /// Permanently remove a portfolio.
    ///
    /// Returns `true` when the portfolio existed and was deleted.
    pub async fn delete_portfolio(&self, id: Uuid) -> Result<bool, PortfolioError> {
        self.service.delete(id).await
    }
}

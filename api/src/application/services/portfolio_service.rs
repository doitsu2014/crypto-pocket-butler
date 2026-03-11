/// PortfolioService — application service for portfolio use cases.
///
/// Coordinates portfolio aggregate operations and account linking.

use std::sync::Arc;
use uuid::Uuid;

use crate::domains::portfolio::{
    aggregate::{Portfolio, PortfolioError},
    repository::PortfolioRepository,
};

/// Application service for portfolio operations.
pub struct PortfolioService {
    repo: Arc<dyn PortfolioRepository>,
}

impl PortfolioService {
    pub fn new(repo: Arc<dyn PortfolioRepository>) -> Self {
        Self { repo }
    }

    /// Create a new portfolio for a user.
    ///
    /// The first portfolio is automatically set as default.
    pub async fn create_portfolio(
        &self,
        user_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Portfolio, PortfolioError> {
        let existing = self.repo.find_by_user_id(user_id).await?;
        let is_default = existing.is_empty();
        let id = Uuid::new_v4();
        let portfolio = Portfolio::new(id, user_id, name, description, is_default);
        self.repo.save(&portfolio).await?;
        Ok(portfolio)
    }

    /// List all portfolios for a user.
    pub async fn list_portfolios(&self, user_id: Uuid) -> Result<Vec<Portfolio>, PortfolioError> {
        self.repo.find_by_user_id(user_id).await
    }

    /// Get a portfolio by ID.
    pub async fn get_portfolio(&self, id: Uuid) -> Result<Portfolio, PortfolioError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(PortfolioError::NotFound)
    }

    /// Add an account to a portfolio.
    pub async fn add_account(
        &self,
        portfolio_id: Uuid,
        account_id: Uuid,
    ) -> Result<(), PortfolioError> {
        let mut portfolio = self.get_portfolio(portfolio_id).await?;
        portfolio.add_account(account_id)?;
        self.repo.save(&portfolio).await
    }

    /// Remove an account from a portfolio.
    pub async fn remove_account(
        &self,
        portfolio_id: Uuid,
        account_id: Uuid,
    ) -> Result<bool, PortfolioError> {
        let mut portfolio = self.get_portfolio(portfolio_id).await?;
        let removed = portfolio.remove_account(account_id);
        if removed {
            self.repo.save(&portfolio).await?;
        }
        Ok(removed)
    }

    /// Delete a portfolio.
    pub async fn delete(&self, portfolio_id: Uuid) -> Result<bool, PortfolioError> {
        self.repo.delete(portfolio_id).await
    }
}

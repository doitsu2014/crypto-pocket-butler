/// PortfolioRepository trait — persistence interface for the Portfolio domain.

use async_trait::async_trait;
use uuid::Uuid;

use super::aggregate::{Portfolio, PortfolioError};

/// Persistence interface for `Portfolio` aggregates.
#[async_trait]
pub trait PortfolioRepository: Send + Sync {
    /// Find a portfolio by primary key.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Portfolio>, PortfolioError>;

    /// Find all portfolios belonging to a user.
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Portfolio>, PortfolioError>;

    /// Find the default portfolio for a user.
    async fn find_default_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Portfolio>, PortfolioError>;

    /// Persist a new or modified portfolio.
    async fn save(&self, portfolio: &Portfolio) -> Result<(), PortfolioError>;

    /// Remove a portfolio by primary key. Returns `true` if it existed.
    async fn delete(&self, id: Uuid) -> Result<bool, PortfolioError>;
}

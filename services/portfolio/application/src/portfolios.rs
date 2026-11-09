//! WF1: portfolios and account membership (BR1.1–BR1.3).

use std::sync::Arc;

use portfolio_domain::{
    AccountId, Portfolio, PortfolioAccount, PortfolioAccountId, PortfolioId, DEFAULT_BASE_CURRENCY,
    DEFAULT_PORTFOLIO_NAME,
};

use crate::common::{AppError, Caller};
use crate::ports::{
    Clock, IdGenerator, MembershipRepository, PortfolioRepository, RepositoryError,
    UserPreferenceClient,
};

/// Lists, creates, and opens portfolios, and assigns accounts.
pub struct PortfolioService {
    portfolios: Arc<dyn PortfolioRepository>,
    memberships: Arc<dyn MembershipRepository>,
    preferences: Arc<dyn UserPreferenceClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl PortfolioService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        portfolios: Arc<dyn PortfolioRepository>,
        memberships: Arc<dyn MembershipRepository>,
        preferences: Arc<dyn UserPreferenceClient>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            portfolios,
            memberships,
            preferences,
            clock,
            ids,
        }
    }

    /// Returns the user's portfolios, creating the default one on demand
    /// (BR1.2).
    ///
    /// # Errors
    /// Returns a repository refusal when the read or the insert fails.
    pub async fn list_or_create(&self, caller: &Caller) -> Result<Vec<Portfolio>, AppError> {
        let existing = self
            .portfolios
            .find_for_user(caller.user_id)
            .await
            .map_err(AppError::from_repository)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        let portfolio = self
            .build(caller, DEFAULT_PORTFOLIO_NAME.to_string(), None)
            .await?;
        match self.portfolios.insert(&portfolio).await {
            Ok(()) => Ok(vec![portfolio]),
            // A concurrent first request may have created the default portfolio
            // between the read and the insert; return what is there instead of
            // failing the caller.
            Err(RepositoryError::Conflict(_)) => self
                .portfolios
                .find_for_user(caller.user_id)
                .await
                .map_err(AppError::from_repository),
            Err(error) => Err(AppError::from_repository(error)),
        }
    }

    /// C8: lists the user's portfolios for the Reporting service without
    /// creating one. Reporting composes a brief only from portfolios that
    /// already exist.
    ///
    /// # Errors
    /// Returns a repository refusal when the read fails.
    pub async fn list_for_user(
        &self,
        user_id: portfolio_domain::UserId,
    ) -> Result<Vec<Portfolio>, AppError> {
        self.portfolios
            .find_for_user(user_id)
            .await
            .map_err(AppError::from_repository)
    }

    /// Creates a portfolio with an optional name and base currency.
    ///
    /// # Errors
    /// Returns a validation refusal for a bad name or currency, and a
    /// repository conflict when the name is already used.
    pub async fn create(
        &self,
        caller: &Caller,
        name: Option<&str>,
        base_currency: Option<&str>,
    ) -> Result<Portfolio, AppError> {
        let name = name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(DEFAULT_PORTFOLIO_NAME)
            .to_string();
        let portfolio = self.build(caller, name, base_currency).await?;
        self.portfolios
            .insert(&portfolio)
            .await
            .map_err(AppError::from_repository)?;
        Ok(portfolio)
    }

    /// Opens one portfolio for its owner (BR1.1).
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown portfolio and
    /// [`AppError::Forbidden`] for another user's portfolio.
    pub async fn owned(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
    ) -> Result<Portfolio, AppError> {
        let portfolio = self
            .portfolios
            .find_by_id(portfolio_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("portfolio"))?;
        caller.require_owner(portfolio.user_id())?;
        Ok(portfolio)
    }

    /// Assigns an account to the portfolio, moving it from any other portfolio
    /// (BR1.3).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's portfolio and a
    /// repository refusal when the write fails.
    pub async fn assign_account(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
        account_id: AccountId,
    ) -> Result<PortfolioAccount, AppError> {
        self.owned(caller, portfolio_id).await?;
        let membership = PortfolioAccount::new(
            PortfolioAccountId::new(self.ids.next()),
            portfolio_id,
            account_id,
            self.clock.now(),
        );
        self.memberships
            .assign(&membership)
            .await
            .map_err(AppError::from_repository)?;
        Ok(membership)
    }

    async fn build(
        &self,
        caller: &Caller,
        name: String,
        base_currency: Option<&str>,
    ) -> Result<Portfolio, AppError> {
        let currency = match base_currency.map(str::trim).filter(|raw| !raw.is_empty()) {
            Some(currency) => currency.to_string(),
            None => match self.preferences.base_currency(caller.user_id).await {
                Ok(Some(currency)) if !currency.trim().is_empty() => currency,
                Ok(_) => DEFAULT_BASE_CURRENCY.to_string(),
                Err(error) => {
                    tracing::warn!(%error, "Identity preference unavailable; using the default currency");
                    DEFAULT_BASE_CURRENCY.to_string()
                }
            },
        };
        Portfolio::new(
            PortfolioId::new(self.ids.next()),
            caller.user_id,
            &name,
            &currency,
            self.clock.now(),
        )
        .map_err(AppError::from_domain)
    }
}

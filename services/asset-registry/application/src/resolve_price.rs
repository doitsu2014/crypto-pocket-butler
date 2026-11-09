//! WF2: resolve a price with priority and fallback (BR2.1, BR2.2).

use std::sync::Arc;

use asset_registry_domain::{Asset, Price, PriceSource};
use chrono::{DateTime, Utc};

use crate::ports::{Clock, PriceProvider, PriceSourceRepository, RepositoryError};

/// A price request for one asset.
#[derive(Debug, Clone)]
pub struct PriceQuery {
    /// The asset to price.
    pub asset: Asset,
}

/// One source that was asked and did not answer with a usable price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceAttempt {
    /// The source name.
    pub source: String,
    /// Why the source did not produce a price.
    pub reason: String,
}

/// The price outcome. There is deliberately no "zero" variant (BR2.2).
#[derive(Debug, Clone)]
pub enum PriceOutcome {
    /// The highest-priority enabled source that answered.
    Priced {
        /// The usable price.
        price: Price,
        /// The source that supplied it.
        source: PriceSource,
        /// When the price was recorded.
        observed_at: DateTime<Utc>,
    },
    /// No source answered; the asset is excluded from priced totals.
    Unpriced {
        /// Every source that was asked, in priority order.
        attempted: Vec<PriceAttempt>,
        /// A human-readable reason.
        reason: String,
    },
}

/// Why price resolution could not run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PriceError {
    /// The source registry failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Resolves prices with priority and fallback.
pub struct ResolvePrice {
    sources: Arc<dyn PriceSourceRepository>,
    provider: Arc<dyn PriceProvider>,
    clock: Arc<dyn Clock>,
}

impl ResolvePrice {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        sources: Arc<dyn PriceSourceRepository>,
        provider: Arc<dyn PriceProvider>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            sources,
            provider,
            clock,
        }
    }

    /// Resolves a price, trying each enabled source in priority order.
    ///
    /// # Errors
    /// Returns [`PriceError::Repository`] when the source registry fails.
    pub async fn execute(&self, query: PriceQuery) -> Result<PriceOutcome, PriceError> {
        let sources = self.sources.list_enabled_by_priority().await?;
        if sources.is_empty() {
            return Ok(PriceOutcome::Unpriced {
                attempted: Vec::new(),
                reason: "no enabled price source".to_string(),
            });
        }

        let mut attempted = Vec::with_capacity(sources.len());
        for source in sources {
            match self.provider.fetch_price(&query.asset, &source).await {
                Ok(Some(price)) => {
                    return Ok(PriceOutcome::Priced {
                        price,
                        source,
                        observed_at: self.clock.now(),
                    });
                }
                Ok(None) => attempted.push(PriceAttempt {
                    source: source.name().to_string(),
                    reason: "no usable price".to_string(),
                }),
                Err(error) => attempted.push(PriceAttempt {
                    source: source.name().to_string(),
                    reason: error.to_string(),
                }),
            }
        }

        Ok(PriceOutcome::Unpriced {
            attempted,
            reason: "every enabled price source failed".to_string(),
        })
    }
}

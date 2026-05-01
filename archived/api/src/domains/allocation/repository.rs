/// Allocation repository trait
///
/// Defines the persistence interface for the allocation bounded context.

use std::error::Error;
use uuid::Uuid;

use super::value_objects::AllocationData;

/// Persistence interface for portfolio allocations.
#[async_trait::async_trait]
pub trait AllocationRepository: Send + Sync {
    /// Persist an allocation for a portfolio, replacing any existing one.
    async fn save(
        &self,
        portfolio_id: Uuid,
        data: &AllocationData,
    ) -> Result<Uuid, Box<dyn Error + Send + Sync>>;

    /// Retrieve the latest allocation for a portfolio.
    async fn find_latest(
        &self,
        portfolio_id: Uuid,
    ) -> Result<Option<AllocationData>, Box<dyn Error + Send + Sync>>;
}

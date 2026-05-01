/// Recommendation use cases — application-layer orchestration for portfolio
/// recommendations.
///
/// # Current state
///
/// The recommendation use cases are thin delegation shells. The full
/// business logic currently lives in the `recommendations.rs` handler and
/// will be migrated here in a follow-up refactor once a `RecommendationRepository`
/// trait is defined in the domain layer.

use sea_orm::DatabaseConnection;

/// Container for all recommendation-related use cases.
pub struct RecommendationUseCases {
    // TODO: Replace with Arc<dyn RecommendationRepository> once the Recommendation
    // domain is fully extracted from the handler into the domain layer.
    // The DatabaseConnection is retained here as a placeholder during the
    // incremental migration.
    #[allow(dead_code)]
    pub(crate) db: DatabaseConnection,
}

impl RecommendationUseCases {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

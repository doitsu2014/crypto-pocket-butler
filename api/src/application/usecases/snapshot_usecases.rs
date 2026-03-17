/// Snapshot use cases — application-layer orchestration for portfolio snapshots.
///
/// HTTP snapshot handlers delegate to these use cases instead of calling
/// the `portfolio_snapshot` job module or SeaORM entities directly.
///
/// # Current state
///
/// The snapshot use cases hold a `DatabaseConnection` directly while a full
/// `SnapshotRepository` trait migration is pending. Once a `Snapshot` domain
/// aggregate and its repository trait are defined, this module should be
/// updated to accept `Arc<dyn SnapshotRepository>` instead.
///
/// TODO: Define `SnapshotRepository` trait in `domains/snapshot/` and update
/// this use case to accept it, consistent with the account/portfolio patterns.

use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::application::jobs::portfolio_snapshot::{
    create_portfolio_snapshot, SnapshotResult,
};

/// Container for all snapshot-related use cases.
pub struct SnapshotUseCases {
    db: DatabaseConnection,
}

impl SnapshotUseCases {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a single snapshot for the given portfolio.
    pub async fn create_snapshot(
        &self,
        portfolio_id: Uuid,
        snapshot_type: &str,
    ) -> Result<SnapshotResult, Box<dyn std::error::Error + Send + Sync>> {
        create_portfolio_snapshot(&self.db, portfolio_id, None, snapshot_type).await
    }
}

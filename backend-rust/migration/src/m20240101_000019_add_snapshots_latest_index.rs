use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add composite index for optimizing latest snapshot queries
        // This index supports the query pattern: WHERE portfolio_id = ? ORDER BY snapshot_date DESC, created_at DESC
        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_latest")
                    .table(Snapshots::Table)
                    .col(Snapshots::PortfolioId)
                    .col((Snapshots::SnapshotDate, IndexOrder::Desc))
                    .col((Snapshots::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_snapshots_latest")
                    .table(Snapshots::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Snapshots {
    Table,
    PortfolioId,
    SnapshotDate,
    CreatedAt,
}

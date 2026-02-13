use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create snapshots table with allocation_id from the start
        manager
            .create_table(
                Table::create()
                    .table(Snapshots::Table)
                    .if_not_exists()
                    .col(uuid(Snapshots::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Snapshots::PortfolioId).not_null())
                    .col(date(Snapshots::SnapshotDate).not_null())
                    .col(string(Snapshots::SnapshotType).not_null()) // "eod", "manual", "hourly"
                    .col(decimal(Snapshots::TotalValueUsd).not_null()) // DECIMAL for precision
                    .col(json(Snapshots::Holdings)) // JSON array of asset holdings
                    .col(json_null(Snapshots::Metadata)) // Optional metadata (exchange rates, etc.)
                    .col(uuid_null(Snapshots::AllocationId)) // Reference to portfolio_allocations (from m000018)
                    .col(timestamp_with_time_zone(Snapshots::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_snapshots_portfolio_id")
                            .from(Snapshots::Table, Snapshots::PortfolioId)
                            .to(Portfolios::Table, Portfolios::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add foreign key to portfolio_allocations (from m000018)
        manager
            .alter_table(
                Table::alter()
                    .table(Snapshots::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_snapshots_allocation_id")
                            .from_tbl(Snapshots::Table)
                            .from_col(Snapshots::AllocationId)
                            .to_tbl(PortfolioAllocations::Table)
                            .to_col(PortfolioAllocations::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on portfolio_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_portfolio_id")
                    .table(Snapshots::Table)
                    .col(Snapshots::PortfolioId)
                    .to_owned(),
            )
            .await?;

        // Create index on snapshot_date for time-series queries
        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_snapshot_date")
                    .table(Snapshots::Table)
                    .col(Snapshots::SnapshotDate)
                    .to_owned(),
            )
            .await?;

        // Create unique index to prevent duplicate snapshots for same portfolio/date/type
        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_unique")
                    .table(Snapshots::Table)
                    .col(Snapshots::PortfolioId)
                    .col(Snapshots::SnapshotDate)
                    .col(Snapshots::SnapshotType)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Add index on allocation_id for faster lookups (from m000018)
        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_allocation_id")
                    .table(Snapshots::Table)
                    .col(Snapshots::AllocationId)
                    .to_owned(),
            )
            .await?;

        // Add composite index for optimizing latest snapshot queries (from m000019)
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
            .drop_table(Table::drop().table(Snapshots::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Snapshots {
    Table,
    Id,
    PortfolioId,
    SnapshotDate,
    SnapshotType,
    TotalValueUsd,
    Holdings,
    Metadata,
    AllocationId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PortfolioAllocations {
    Table,
    Id,
}

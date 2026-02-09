use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
    CreatedAt,
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
}

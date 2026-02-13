use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create portfolio_allocations table with unique constraint from the start
        manager
            .create_table(
                Table::create()
                    .table(PortfolioAllocations::Table)
                    .if_not_exists()
                    .col(uuid(PortfolioAllocations::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(PortfolioAllocations::PortfolioId).not_null())
                    .col(timestamp_with_time_zone(PortfolioAllocations::AsOf).not_null()) // Time of allocation snapshot
                    .col(decimal(PortfolioAllocations::TotalValueUsd).not_null()) // Total portfolio value in USD
                    .col(json(PortfolioAllocations::Holdings).not_null()) // JSON array of asset holdings with values and weights
                    .col(timestamp_with_time_zone(PortfolioAllocations::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolio_allocations_portfolio_id")
                            .from(PortfolioAllocations::Table, PortfolioAllocations::PortfolioId)
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
                    .name("idx_portfolio_allocations_portfolio_id")
                    .table(PortfolioAllocations::Table)
                    .col(PortfolioAllocations::PortfolioId)
                    .to_owned(),
            )
            .await?;

        // Create composite index for efficient time-series queries per portfolio
        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_allocations_portfolio_as_of")
                    .table(PortfolioAllocations::Table)
                    .col(PortfolioAllocations::PortfolioId)
                    .col(PortfolioAllocations::AsOf)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint on portfolio_id to ensure only one allocation per portfolio (from m20240101_000017)
        manager
            .create_index(
                Index::create()
                    .name("uq_portfolio_allocations_portfolio_id")
                    .table(PortfolioAllocations::Table)
                    .col(PortfolioAllocations::PortfolioId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PortfolioAllocations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PortfolioAllocations {
    Table,
    Id,
    PortfolioId,
    AsOf,
    TotalValueUsd,
    Holdings,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
}

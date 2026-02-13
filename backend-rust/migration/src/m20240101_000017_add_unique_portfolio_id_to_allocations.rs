use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add unique constraint on portfolio_id to ensure only one allocation per portfolio
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
            .drop_index(
                Index::drop()
                    .name("uq_portfolio_allocations_portfolio_id")
                    .table(PortfolioAllocations::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum PortfolioAllocations {
    Table,
    PortfolioId,
}

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Before adding unique constraint, we need to handle any existing duplicates
        // Keep only the most recent allocation per portfolio_id (by as_of timestamp)
        let db = manager.get_connection();
        
        // Delete older duplicates, keeping only the latest one per portfolio_id
        // Note: This SQL uses DISTINCT ON which is PostgreSQL-specific syntax
        let delete_sql = r#"
            DELETE FROM portfolio_allocations
            WHERE id NOT IN (
                SELECT DISTINCT ON (portfolio_id) id
                FROM portfolio_allocations
                ORDER BY portfolio_id, as_of DESC
            )
        "#;
        
        db.execute_unprepared(delete_sql).await?;
        
        // Now add unique constraint on portfolio_id to ensure only one allocation per portfolio
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

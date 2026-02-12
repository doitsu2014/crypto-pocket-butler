use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Recommendations::Table)
                    .if_not_exists()
                    .col(uuid(Recommendations::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Recommendations::PortfolioId).not_null())
                    .col(string(Recommendations::Status).not_null()) // "pending", "approved", "rejected", "executed"
                    .col(string(Recommendations::RecommendationType).not_null()) // "rebalance", "take_profit", "stop_loss"
                    .col(text(Recommendations::Rationale).not_null()) // Why this recommendation was made
                    .col(json(Recommendations::ProposedOrders)) // Array of order objects
                    .col(decimal_null(Recommendations::ExpectedImpact)) // Expected value impact (USD)
                    .col(json_null(Recommendations::Metadata)) // Additional data (risk score, confidence, etc.)
                    .col(timestamp_with_time_zone(Recommendations::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Recommendations::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone_null(Recommendations::ExecutedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recommendations_portfolio_id")
                            .from(Recommendations::Table, Recommendations::PortfolioId)
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
                    .name("idx_recommendations_portfolio_id")
                    .table(Recommendations::Table)
                    .col(Recommendations::PortfolioId)
                    .to_owned(),
            )
            .await?;

        // Create index on status for filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_recommendations_status")
                    .table(Recommendations::Table)
                    .col(Recommendations::Status)
                    .to_owned(),
            )
            .await?;

        // Create index on created_at for time-series queries
        manager
            .create_index(
                Index::create()
                    .name("idx_recommendations_created_at")
                    .table(Recommendations::Table)
                    .col(Recommendations::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Recommendations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Recommendations {
    Table,
    Id,
    PortfolioId,
    Status,
    RecommendationType,
    Rationale,
    ProposedOrders,
    ExpectedImpact,
    Metadata,
    CreatedAt,
    UpdatedAt,
    ExecutedAt,
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
}

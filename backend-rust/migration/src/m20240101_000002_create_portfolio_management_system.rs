use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create portfolios table with all columns including settings and last_constructed_at
        manager
            .create_table(
                Table::create()
                    .table(Portfolios::Table)
                    .if_not_exists()
                    .col(uuid(Portfolios::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Portfolios::UserId).not_null())
                    .col(string(Portfolios::Name).not_null())
                    .col(text_null(Portfolios::Description))
                    .col(boolean(Portfolios::IsDefault).default(false).not_null())
                    .col(json_null(Portfolios::TargetAllocation)) // Added from m000008
                    .col(json_null(Portfolios::Guardrails)) // Added from m000008
                    .col(timestamp_with_time_zone_null(Portfolios::LastConstructedAt)) // Added from m000016
                    .col(timestamp_with_time_zone(Portfolios::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Portfolios::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolios_user_id")
                            .from(Portfolios::Table, Portfolios::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_portfolios_user_id")
                    .table(Portfolios::Table)
                    .col(Portfolios::UserId)
                    .to_owned(),
            )
            .await?;

        // Create unique index to ensure only one default portfolio per user
        // Using raw SQL for partial unique index (WHERE is_default = true)
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_portfolios_user_id_is_default \
                 ON portfolios (user_id) WHERE is_default = true"
            )
            .await?;

        // Create portfolio_accounts junction table
        manager
            .create_table(
                Table::create()
                    .table(PortfolioAccounts::Table)
                    .if_not_exists()
                    .col(uuid(PortfolioAccounts::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(PortfolioAccounts::PortfolioId).not_null())
                    .col(uuid(PortfolioAccounts::AccountId).not_null())
                    .col(timestamp_with_time_zone(PortfolioAccounts::AddedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolio_accounts_portfolio_id")
                            .from(PortfolioAccounts::Table, PortfolioAccounts::PortfolioId)
                            .to(Portfolios::Table, Portfolios::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolio_accounts_account_id")
                            .from(PortfolioAccounts::Table, PortfolioAccounts::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index to prevent duplicate account-portfolio associations
        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_accounts_unique")
                    .table(PortfolioAccounts::Table)
                    .col(PortfolioAccounts::PortfolioId)
                    .col(PortfolioAccounts::AccountId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create index on portfolio_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_accounts_portfolio_id")
                    .table(PortfolioAccounts::Table)
                    .col(PortfolioAccounts::PortfolioId)
                    .to_owned(),
            )
            .await?;

        // Create index on account_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_accounts_account_id")
                    .table(PortfolioAccounts::Table)
                    .col(PortfolioAccounts::AccountId)
                    .to_owned(),
            )
            .await?;

        // Create recommendations table
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
                    .col(json(Recommendations::ProposedOrders).not_null()) // Array of order objects
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
            .await?;
        
        manager
            .drop_table(Table::drop().table(PortfolioAccounts::Table).to_owned())
            .await?;
        
        manager
            .drop_table(Table::drop().table(Portfolios::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
    UserId,
    Name,
    Description,
    IsDefault,
    TargetAllocation,
    Guardrails,
    LastConstructedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PortfolioAccounts {
    Table,
    Id,
    PortfolioId,
    AccountId,
    AddedAt,
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

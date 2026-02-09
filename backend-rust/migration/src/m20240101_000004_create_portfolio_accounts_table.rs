use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PortfolioAccounts::Table).to_owned())
            .await
    }
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
enum Portfolios {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

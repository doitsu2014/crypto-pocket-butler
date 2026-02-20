use sea_orm_migration::{prelude::*, schema::*};

/// Consolidated migration: creates the entire core user/account/portfolio system.
///
/// Tables created:
/// 1. users – Keycloak-backed user profiles
/// 2. accounts – exchange/wallet/DeFi accounts belonging to a user
/// 3. portfolios – named collections of accounts owned by a user
/// 4. portfolio_accounts – junction table linking portfolios ↔ accounts
/// 5. recommendations – AI-generated portfolio rebalancing suggestions
/// 6. portfolio_allocations – current allocation snapshot per portfolio
/// 7. snapshots – historical portfolio value snapshots (EOD, manual, hourly)
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── 1. users ──────────────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(uuid(Users::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(string(Users::KeycloakUserId).unique_key().not_null())
                    .col(string_null(Users::Email))
                    .col(string_null(Users::PreferredUsername))
                    .col(timestamp_with_time_zone(Users::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Users::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_users_keycloak_user_id")
                    .table(Users::Table)
                    .col(Users::KeycloakUserId)
                    .to_owned(),
            )
            .await?;

        // ── 2. accounts ───────────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(uuid(Accounts::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Accounts::UserId).not_null())
                    .col(string(Accounts::Name).not_null())
                    .col(string(Accounts::AccountType).not_null())
                    .col(string_null(Accounts::ExchangeName))
                    .col(string_null(Accounts::ApiKeyEncrypted))
                    .col(string_null(Accounts::ApiSecretEncrypted))
                    .col(string_null(Accounts::PassphraseEncrypted))
                    .col(string_null(Accounts::WalletAddress))
                    .col(boolean(Accounts::IsActive).default(true).not_null())
                    .col(timestamp_with_time_zone_null(Accounts::LastSyncedAt))
                    .col(json_null(Accounts::Holdings))
                    .col(json_null(Accounts::EnabledChains))
                    .col(timestamp_with_time_zone(Accounts::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Accounts::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_user_id")
                            .from(Accounts::Table, Accounts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_accounts_user_id")
                    .table(Accounts::Table)
                    .col(Accounts::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_accounts_account_type")
                    .table(Accounts::Table)
                    .col(Accounts::AccountType)
                    .to_owned(),
            )
            .await?;

        // ── 3. portfolios ─────────────────────────────────────────────────────
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
                    .col(json_null(Portfolios::TargetAllocation))
                    .col(json_null(Portfolios::Guardrails))
                    .col(timestamp_with_time_zone_null(Portfolios::LastConstructedAt))
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

        manager
            .create_index(
                Index::create()
                    .name("idx_portfolios_user_id")
                    .table(Portfolios::Table)
                    .col(Portfolios::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_portfolios_user_id_is_default \
                 ON portfolios (user_id) WHERE is_default = true",
            )
            .await?;

        // ── 4. portfolio_accounts ─────────────────────────────────────────────
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

        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_accounts_portfolio_id")
                    .table(PortfolioAccounts::Table)
                    .col(PortfolioAccounts::PortfolioId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_accounts_account_id")
                    .table(PortfolioAccounts::Table)
                    .col(PortfolioAccounts::AccountId)
                    .to_owned(),
            )
            .await?;

        // ── 5. recommendations ────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Recommendations::Table)
                    .if_not_exists()
                    .col(uuid(Recommendations::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Recommendations::PortfolioId).not_null())
                    .col(string(Recommendations::Status).not_null())
                    .col(string(Recommendations::RecommendationType).not_null())
                    .col(text(Recommendations::Rationale).not_null())
                    .col(json(Recommendations::ProposedOrders).not_null())
                    .col(decimal_null(Recommendations::ExpectedImpact))
                    .col(json_null(Recommendations::Metadata))
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

        manager
            .create_index(
                Index::create()
                    .name("idx_recommendations_portfolio_id")
                    .table(Recommendations::Table)
                    .col(Recommendations::PortfolioId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_recommendations_status")
                    .table(Recommendations::Table)
                    .col(Recommendations::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_recommendations_created_at")
                    .table(Recommendations::Table)
                    .col(Recommendations::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // ── 6. portfolio_allocations ──────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(PortfolioAllocations::Table)
                    .if_not_exists()
                    .col(uuid(PortfolioAllocations::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(PortfolioAllocations::PortfolioId).not_null())
                    .col(timestamp_with_time_zone(PortfolioAllocations::AsOf).not_null())
                    .col(decimal(PortfolioAllocations::TotalValueUsd).not_null())
                    .col(json(PortfolioAllocations::Holdings).not_null())
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

        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_allocations_portfolio_id")
                    .table(PortfolioAllocations::Table)
                    .col(PortfolioAllocations::PortfolioId)
                    .to_owned(),
            )
            .await?;

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

        manager
            .create_index(
                Index::create()
                    .name("uq_portfolio_allocations_portfolio_id")
                    .table(PortfolioAllocations::Table)
                    .col(PortfolioAllocations::PortfolioId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ── 7. snapshots ──────────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Snapshots::Table)
                    .if_not_exists()
                    .col(uuid(Snapshots::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Snapshots::PortfolioId).not_null())
                    .col(date(Snapshots::SnapshotDate).not_null())
                    .col(string(Snapshots::SnapshotType).not_null())
                    .col(decimal(Snapshots::TotalValueUsd).not_null())
                    .col(json(Snapshots::Holdings))
                    .col(json_null(Snapshots::Metadata))
                    .col(uuid_null(Snapshots::AllocationId))
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

        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_portfolio_id")
                    .table(Snapshots::Table)
                    .col(Snapshots::PortfolioId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_snapshot_date")
                    .table(Snapshots::Table)
                    .col(Snapshots::SnapshotDate)
                    .to_owned(),
            )
            .await?;

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

        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_allocation_id")
                    .table(Snapshots::Table)
                    .col(Snapshots::AllocationId)
                    .to_owned(),
            )
            .await?;

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
            .await?;

        manager
            .drop_table(Table::drop().table(PortfolioAllocations::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Recommendations::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(PortfolioAccounts::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Portfolios::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Accounts::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    KeycloakUserId,
    Email,
    PreferredUsername,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
    UserId,
    Name,
    AccountType,
    ExchangeName,
    ApiKeyEncrypted,
    ApiSecretEncrypted,
    PassphraseEncrypted,
    WalletAddress,
    IsActive,
    LastSyncedAt,
    Holdings,
    EnabledChains,
    CreatedAt,
    UpdatedAt,
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

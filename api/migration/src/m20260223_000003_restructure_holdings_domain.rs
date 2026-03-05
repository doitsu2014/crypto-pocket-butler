use sea_orm_migration::{prelude::*, schema::*};

/// Migration: restructure holdings into proper domain separation.
///
/// ## Domain model after this migration
///
/// ### Accounts domain (Wallet / Exchange)
/// `account_transactions` – every balance observation captured during a sync.
/// The current balance for `(account_id, asset_symbol)` is the `quantity_after`
/// of the most-recent transaction row for that pair.
///
/// ### Portfolios domain
/// `portfolio_holdings` – one row per `(portfolio_id, asset_symbol)` representing
/// the current aggregated quantity across all accounts linked to the portfolio.
///
/// `holding_valuations` – append-only daily valuation records keyed to a portfolio
/// holding.  Used to reconstruct or analyse portfolio performance over time.
///
/// ### Infrastructure domain (unchanged)
/// `assets`, `asset_prices` – remain as-is.
///
/// ## Steps
/// 1. Drop `holding_transactions` (from m20260223_000002).
/// 2. Drop `holdings`            (from m20260223_000002).
/// 3. Create `account_transactions`.
/// 4. Create `portfolio_holdings`.
/// 5. Create `holding_valuations`.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── Drop legacy tables from previous migration ─────────────────────────
        manager
            .drop_table(Table::drop().table(HoldingTransactions::Table).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Holdings::Table).if_exists().to_owned())
            .await?;

        // ── 1. account_transactions ────────────────────────────────────────────
        // Append-only audit log in the Accounts domain.
        // Each row represents a balance observation for one asset in one account.
        manager
            .create_table(
                Table::create()
                    .table(AccountTransactions::Table)
                    .if_not_exists()
                    .col(uuid(AccountTransactions::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AccountTransactions::AccountId).not_null())
                    // Asset symbol as reported by the exchange or chain (e.g. "BTC", "ETH-ethereum")
                    .col(string(AccountTransactions::AssetSymbol).not_null())
                    // Balance *before* this observation (normalized decimal string)
                    .col(string(AccountTransactions::QuantityBefore).not_null().default("0"))
                    // Balance *after* this observation (normalized decimal string)
                    .col(string(AccountTransactions::QuantityAfter).not_null())
                    // Signed delta: quantity_after − quantity_before
                    .col(string(AccountTransactions::QuantityChange).not_null())
                    // Semantic type: "sync" | "deposit" | "withdrawal" | "manual_adjustment"
                    .col(string(AccountTransactions::TransactionType).not_null().default("sync"))
                    // Origin: "okx" | "ethereum" | "solana" | "manual" | …
                    .col(string(AccountTransactions::Source).not_null().default("sync"))
                    // Freeform audit context (job ID, triggering user, …)
                    .col(json_null(AccountTransactions::Metadata))
                    .col(timestamp_with_time_zone(AccountTransactions::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(AccountTransactions::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_account_transactions_account_id")
                            .from(AccountTransactions::Table, AccountTransactions::AccountId)
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
                    .name("idx_account_transactions_account_id")
                    .table(AccountTransactions::Table)
                    .col(AccountTransactions::AccountId)
                    .to_owned(),
            )
            .await?;

        // Composite index to efficiently find the latest transaction for a given
        // (account_id, asset_symbol) pair – equivalent to "current balance".
        manager
            .create_index(
                Index::create()
                    .name("idx_account_transactions_account_asset_created")
                    .table(AccountTransactions::Table)
                    .col(AccountTransactions::AccountId)
                    .col(AccountTransactions::AssetSymbol)
                    .col((AccountTransactions::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_account_transactions_created_at")
                    .table(AccountTransactions::Table)
                    .col(AccountTransactions::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // ── 2. portfolio_holdings ──────────────────────────────────────────────
        // Portfolios domain: aggregated current holding per (portfolio, asset).
        manager
            .create_table(
                Table::create()
                    .table(PortfolioHoldings::Table)
                    .if_not_exists()
                    .col(uuid(PortfolioHoldings::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(PortfolioHoldings::PortfolioId).not_null())
                    // Canonical asset symbol (e.g. "BTC", "ETH")
                    .col(string(PortfolioHoldings::AssetSymbol).not_null())
                    // Current aggregated quantity across all linked accounts (normalized decimal)
                    .col(string(PortfolioHoldings::Quantity).not_null().default("0"))
                    .col(timestamp_with_time_zone(PortfolioHoldings::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(PortfolioHoldings::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolio_holdings_portfolio_id")
                            .from(PortfolioHoldings::Table, PortfolioHoldings::PortfolioId)
                            .to(Portfolios::Table, Portfolios::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // One holding row per (portfolio_id, asset_symbol)
        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_holdings_portfolio_asset_unique")
                    .table(PortfolioHoldings::Table)
                    .col(PortfolioHoldings::PortfolioId)
                    .col(PortfolioHoldings::AssetSymbol)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_portfolio_holdings_portfolio_id")
                    .table(PortfolioHoldings::Table)
                    .col(PortfolioHoldings::PortfolioId)
                    .to_owned(),
            )
            .await?;

        // ── 3. holding_valuations ──────────────────────────────────────────────
        // Portfolios domain: daily time-series valuation for each portfolio holding.
        manager
            .create_table(
                Table::create()
                    .table(HoldingValuations::Table)
                    .if_not_exists()
                    .col(uuid(HoldingValuations::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(HoldingValuations::PortfolioHoldingId).not_null())
                    // Calendar date of this valuation snapshot
                    .col(date(HoldingValuations::Date).not_null())
                    // Asset price used for this valuation (from asset_prices)
                    .col(decimal(HoldingValuations::PriceUsd).not_null())
                    // Quantity of the holding on this date (normalized decimal)
                    .col(decimal(HoldingValuations::Quantity).not_null())
                    // Computed value: price_usd * quantity
                    .col(decimal(HoldingValuations::ValueUsd).not_null())
                    .col(timestamp_with_time_zone(HoldingValuations::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_holding_valuations_portfolio_holding_id")
                            .from(HoldingValuations::Table, HoldingValuations::PortfolioHoldingId)
                            .to(PortfolioHoldings::Table, PortfolioHoldings::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // One valuation per (holding, date) – prevents double-recording on re-runs
        manager
            .create_index(
                Index::create()
                    .name("idx_holding_valuations_holding_date_unique")
                    .table(HoldingValuations::Table)
                    .col(HoldingValuations::PortfolioHoldingId)
                    .col(HoldingValuations::Date)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_holding_valuations_date")
                    .table(HoldingValuations::Table)
                    .col(HoldingValuations::Date)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HoldingValuations::Table).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(PortfolioHoldings::Table).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AccountTransactions::Table).if_exists().to_owned())
            .await?;

        // Recreate the legacy tables so that down() fully restores m20260223_000002
        manager
            .create_table(
                Table::create()
                    .table(Holdings::Table)
                    .if_not_exists()
                    .col(uuid(Holdings::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Holdings::AccountId).not_null())
                    .col(string(Holdings::AssetSymbol).not_null())
                    .col(string(Holdings::Quantity).not_null().default("0"))
                    .col(timestamp_with_time_zone(Holdings::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Holdings::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_holdings_account_id")
                            .from(Holdings::Table, Holdings::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HoldingTransactions::Table)
                    .if_not_exists()
                    .col(uuid(HoldingTransactions::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(HoldingTransactions::HoldingId).not_null())
                    .col(string(HoldingTransactions::QuantityBefore).not_null().default("0"))
                    .col(string(HoldingTransactions::QuantityAfter).not_null())
                    .col(string(HoldingTransactions::QuantityChange).not_null())
                    .col(string(HoldingTransactions::TransactionType).not_null().default("sync"))
                    .col(string(HoldingTransactions::Source).not_null().default("sync"))
                    .col(json_null(HoldingTransactions::Metadata))
                    .col(timestamp_with_time_zone(HoldingTransactions::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(HoldingTransactions::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_holding_transactions_holding_id")
                            .from(HoldingTransactions::Table, HoldingTransactions::HoldingId)
                            .to(Holdings::Table, Holdings::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
}

// ── Iden enums ─────────────────────────────────────────────────────────────────

#[derive(DeriveIden)]
enum AccountTransactions {
    Table,
    Id,
    AccountId,
    AssetSymbol,
    QuantityBefore,
    QuantityAfter,
    QuantityChange,
    TransactionType,
    Source,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PortfolioHoldings {
    Table,
    Id,
    PortfolioId,
    AssetSymbol,
    Quantity,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum HoldingValuations {
    Table,
    Id,
    PortfolioHoldingId,
    Date,
    PriceUsd,
    Quantity,
    ValueUsd,
    CreatedAt,
}

// Referenced only for FK definitions
#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
}

// Legacy tables (only used in down())
#[derive(DeriveIden)]
enum Holdings {
    Table,
    Id,
    AccountId,
    AssetSymbol,
    Quantity,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum HoldingTransactions {
    Table,
    Id,
    HoldingId,
    QuantityBefore,
    QuantityAfter,
    QuantityChange,
    TransactionType,
    Source,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

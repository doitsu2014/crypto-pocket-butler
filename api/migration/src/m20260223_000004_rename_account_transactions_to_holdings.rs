use sea_orm_migration::{prelude::*, schema::*};

/// Migration: replace `account_transactions` with `account_holdings`.
///
/// ## Change in semantics
///
/// `account_transactions` (introduced in m20260223_000003) was an append-only
/// log.  The reviewer clarified that the Accounts domain should hold the
/// **current** balance for each asset – one row per `(account_id, asset_symbol)`
/// that is **upserted** (overwritten) on every sync.
///
/// ## Result
/// `account_holdings` — one row per `(account_id, asset_symbol)`:
/// * `quantity`    – current normalized balance (overwritten on each sync)
/// * `source`      – data source that produced the latest value
/// * `created_at`  – when this holding was first seen
/// * `updated_at`  – when the balance was last updated
///
/// A unique index on `(account_id, asset_symbol)` enforces the one-row
/// invariant and allows efficient lookups.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the old append-only log
        manager
            .drop_table(Table::drop().table(AccountTransactions::Table).if_exists().to_owned())
            .await?;

        // Create the new current-state holdings table
        manager
            .create_table(
                Table::create()
                    .table(AccountHoldings::Table)
                    .if_not_exists()
                    .col(uuid(AccountHoldings::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AccountHoldings::AccountId).not_null())
                    // Asset symbol as reported by the exchange/chain (e.g. "BTC", "ETH-ethereum")
                    .col(string(AccountHoldings::AssetSymbol).not_null())
                    // Current normalized quantity (overwritten on each sync)
                    .col(string(AccountHoldings::Quantity).not_null().default("0"))
                    // Data source that produced the latest value
                    .col(string(AccountHoldings::Source).not_null().default("sync"))
                    .col(timestamp_with_time_zone(AccountHoldings::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(AccountHoldings::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_account_holdings_account_id")
                            .from(AccountHoldings::Table, AccountHoldings::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // One row per (account_id, asset_symbol)
        manager
            .create_index(
                Index::create()
                    .name("idx_account_holdings_account_asset_unique")
                    .table(AccountHoldings::Table)
                    .col(AccountHoldings::AccountId)
                    .col(AccountHoldings::AssetSymbol)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_account_holdings_account_id")
                    .table(AccountHoldings::Table)
                    .col(AccountHoldings::AccountId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AccountHoldings::Table).if_exists().to_owned())
            .await?;

        // Recreate account_transactions so down() fully restores m20260223_000003
        manager
            .create_table(
                Table::create()
                    .table(AccountTransactions::Table)
                    .if_not_exists()
                    .col(uuid(AccountTransactions::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AccountTransactions::AccountId).not_null())
                    .col(string(AccountTransactions::AssetSymbol).not_null())
                    .col(string(AccountTransactions::QuantityBefore).not_null().default("0"))
                    .col(string(AccountTransactions::QuantityAfter).not_null())
                    .col(string(AccountTransactions::QuantityChange).not_null())
                    .col(string(AccountTransactions::TransactionType).not_null().default("sync"))
                    .col(string(AccountTransactions::Source).not_null().default("sync"))
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
            .await
    }
}

#[derive(DeriveIden)]
enum AccountHoldings {
    Table,
    Id,
    AccountId,
    AssetSymbol,
    Quantity,
    Source,
    CreatedAt,
    UpdatedAt,
}

// Referenced only for the FK definition and down() recreation
#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

// Used only in down() to recreate the previous table
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

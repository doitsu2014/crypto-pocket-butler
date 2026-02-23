use sea_orm_migration::{prelude::*, schema::*};

/// Migration: create `holdings` and `holding_transactions` tables.
///
/// Design:
///   Account → `holdings` (one row per asset per account, current state)
///           → `holding_transactions` (append-only audit log; replaying all rows
///              for a holding always reproduces the current `holdings.quantity`)
///
/// `holding_transactions` captures every balance change detected during account
/// syncs, including full audit metadata (`created_at`, `updated_at`, `source`,
/// `transaction_type`, `metadata`).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── 1. holdings ───────────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Holdings::Table)
                    .if_not_exists()
                    .col(uuid(Holdings::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Holdings::AccountId).not_null())
                    // Asset symbol as stored on the exchange/chain (e.g. "BTC", "ETH-ethereum")
                    .col(string(Holdings::AssetSymbol).not_null())
                    // Current reconstructed quantity (sum of all transactions)
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

        // Unique: one holding row per (account_id, asset_symbol)
        manager
            .create_index(
                Index::create()
                    .name("idx_holdings_account_asset_unique")
                    .table(Holdings::Table)
                    .col(Holdings::AccountId)
                    .col(Holdings::AssetSymbol)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_holdings_account_id")
                    .table(Holdings::Table)
                    .col(Holdings::AccountId)
                    .to_owned(),
            )
            .await?;

        // ── 2. holding_transactions ───────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(HoldingTransactions::Table)
                    .if_not_exists()
                    .col(uuid(HoldingTransactions::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(HoldingTransactions::HoldingId).not_null())
                    // Snapshot of the balance *before* this event
                    .col(string(HoldingTransactions::QuantityBefore).not_null().default("0"))
                    // Snapshot of the balance *after* this event
                    .col(string(HoldingTransactions::QuantityAfter).not_null())
                    // Signed delta: quantity_after − quantity_before
                    .col(string(HoldingTransactions::QuantityChange).not_null())
                    // e.g. "sync", "deposit", "withdrawal", "manual_adjustment"
                    .col(string(HoldingTransactions::TransactionType).not_null().default("sync"))
                    // Data source (e.g. "okx", "ethereum", "manual")
                    .col(string(HoldingTransactions::Source).not_null().default("sync"))
                    // Optional freeform audit metadata (triggering user, job ID, etc.)
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
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_holding_transactions_holding_id")
                    .table(HoldingTransactions::Table)
                    .col(HoldingTransactions::HoldingId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_holding_transactions_created_at")
                    .table(HoldingTransactions::Table)
                    .col(HoldingTransactions::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HoldingTransactions::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Holdings::Table).to_owned())
            .await
    }
}

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

// Referenced only for the FK definition
#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

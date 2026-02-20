use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `evm_tokens` table.
///
/// This table stores the configurable list of ERC-20 token addresses that the EVM connector
/// will check during account synchronisation. Rows can be managed via the `/api/v1/evm-tokens`
/// CRUD API or auto-populated from the `asset_contracts` table which is kept up to date by the
/// `fetch_all_coins` background job.
///
/// The `is_active` flag lets operators disable a token without deleting the row.
/// The unique index on `(chain, contract_address)` prevents duplicates.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EvmTokens::Table)
                    .if_not_exists()
                    .col(
                        uuid(EvmTokens::Id)
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(string(EvmTokens::Chain).not_null()) // e.g. "ethereum", "arbitrum", "bsc"
                    .col(string(EvmTokens::Symbol).not_null()) // e.g. "USDC", "WBTC"
                    .col(string(EvmTokens::ContractAddress).not_null()) // checksummed hex address
                    .col(boolean(EvmTokens::IsActive).default(true).not_null())
                    .col(
                        timestamp_with_time_zone(EvmTokens::CreatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        timestamp_with_time_zone(EvmTokens::UpdatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: one row per (chain, contract_address) pair
        manager
            .create_index(
                Index::create()
                    .name("idx_evm_tokens_chain_address_unique")
                    .table(EvmTokens::Table)
                    .col(EvmTokens::Chain)
                    .col(EvmTokens::ContractAddress)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index for fast per-chain lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_evm_tokens_chain")
                    .table(EvmTokens::Table)
                    .col(EvmTokens::Chain)
                    .to_owned(),
            )
            .await?;

        // Index for active-flag lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_evm_tokens_is_active")
                    .table(EvmTokens::Table)
                    .col(EvmTokens::IsActive)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EvmTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EvmTokens {
    Table,
    Id,
    Chain,
    Symbol,
    ContractAddress,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

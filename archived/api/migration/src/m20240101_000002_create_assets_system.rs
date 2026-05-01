use sea_orm_migration::{prelude::*, schema::*};

/// Consolidated migration: creates the entire assets/prices system in its final state.
///
/// Tables created:
/// 1. assets        – tradable assets tracked by the system; uses `coinpaprika_id` and a
///                    composite unique index on (symbol, name)
/// 2. asset_contracts – on-chain contract addresses per asset per chain
/// 3. asset_prices  – time-series price snapshots with full CoinPaprika fields
///                    (rank, supply, ATH, percent-change variants) and correct column
///                    names (e.g. `volume_24h_usd`, `change_percent_24h`)
///
/// Note: the `asset_rankings` table is intentionally omitted; that data has been
/// consolidated into `asset_prices` (rank column).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── 1. assets ─────────────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Assets::Table)
                    .if_not_exists()
                    .col(uuid(Assets::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(string(Assets::Symbol).not_null())
                    .col(string(Assets::Name).not_null())
                    .col(string(Assets::AssetType).not_null())
                    .col(string_null(Assets::CoinpaprikaId))
                    .col(string_null(Assets::CoinmarketcapId))
                    .col(string_null(Assets::LogoUrl))
                    .col(text_null(Assets::Description))
                    .col(integer_null(Assets::Decimals))
                    .col(boolean(Assets::IsActive).default(true).not_null())
                    .col(timestamp_with_time_zone(Assets::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Assets::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .to_owned(),
            )
            .await?;

        // Composite unique index on (symbol, name) – allows the same ticker to be
        // used by multiple projects as long as the full name is distinct.
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_symbol_name_unique")
                    .table(Assets::Table)
                    .col(Assets::Symbol)
                    .col(Assets::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assets_asset_type")
                    .table(Assets::Table)
                    .col(Assets::AssetType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assets_coinpaprika_id")
                    .table(Assets::Table)
                    .col(Assets::CoinpaprikaId)
                    .to_owned(),
            )
            .await?;

        // ── 2. asset_contracts ────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(AssetContracts::Table)
                    .if_not_exists()
                    .col(uuid(AssetContracts::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AssetContracts::AssetId).not_null())
                    .col(string(AssetContracts::Chain).not_null())
                    .col(string(AssetContracts::ContractAddress).not_null())
                    .col(string_null(AssetContracts::TokenStandard))
                    .col(integer_null(AssetContracts::Decimals))
                    .col(boolean(AssetContracts::IsVerified).default(false).not_null())
                    .col(timestamp_with_time_zone(AssetContracts::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(AssetContracts::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_contracts_asset_id")
                            .from(AssetContracts::Table, AssetContracts::AssetId)
                            .to(Assets::Table, Assets::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_asset_id")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::AssetId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_chain")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::Chain)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_unique")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::Chain)
                    .col(AssetContracts::ContractAddress)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ── 3. asset_prices ───────────────────────────────────────────────────
        // Created in final state: includes all CoinPaprika extended fields, and uses
        // the correct column names with underscores before numeric suffixes
        // (e.g. volume_24h_usd, change_percent_24h, percent_change_1h, etc.)
        manager
            .create_table(
                Table::create()
                    .table(AssetPrices::Table)
                    .if_not_exists()
                    .col(uuid(AssetPrices::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AssetPrices::AssetId).not_null())
                    .col(timestamp_with_time_zone(AssetPrices::Timestamp).not_null())
                    .col(decimal(AssetPrices::PriceUsd).not_null())
                    // Use Alias::new for columns whose names contain numeric segments to
                    // avoid the DeriveIden snake_case ambiguity with numbers.
                    .col(ColumnDef::new(Alias::new("volume_24h_usd")).decimal().null())
                    .col(decimal_null(AssetPrices::MarketCapUsd))
                    .col(ColumnDef::new(Alias::new("change_percent_24h")).decimal().null())
                    .col(string(AssetPrices::Source).not_null())
                    .col(timestamp_with_time_zone(AssetPrices::CreatedAt).default(Expr::current_timestamp()).not_null())
                    // CoinPaprika extended fields
                    .col(integer_null(AssetPrices::Rank))
                    .col(decimal_null(AssetPrices::CirculatingSupply))
                    .col(decimal_null(AssetPrices::TotalSupply))
                    .col(decimal_null(AssetPrices::MaxSupply))
                    .col(decimal_null(AssetPrices::BetaValue))
                    .col(ColumnDef::new(Alias::new("percent_change_1h")).decimal().null())
                    .col(ColumnDef::new(Alias::new("percent_change_7d")).decimal().null())
                    .col(ColumnDef::new(Alias::new("percent_change_30d")).decimal().null())
                    .col(decimal_null(AssetPrices::AthPrice))
                    .col(timestamp_with_time_zone_null(AssetPrices::AthDate))
                    .col(decimal_null(AssetPrices::PercentFromPriceAth))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_prices_asset_id")
                            .from(AssetPrices::Table, AssetPrices::AssetId)
                            .to(Assets::Table, Assets::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_asset_id")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::AssetId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_timestamp")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_asset_timestamp")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::AssetId)
                    .col(AssetPrices::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_unique")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::AssetId)
                    .col(AssetPrices::Timestamp)
                    .col(AssetPrices::Source)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_rank")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::Rank)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_timestamp_rank")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::Timestamp)
                    .col(AssetPrices::Rank)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AssetPrices::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AssetContracts::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Assets::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
    Symbol,
    Name,
    AssetType,
    CoinpaprikaId,
    CoinmarketcapId,
    LogoUrl,
    Description,
    Decimals,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AssetContracts {
    Table,
    Id,
    AssetId,
    Chain,
    ContractAddress,
    TokenStandard,
    Decimals,
    IsVerified,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AssetPrices {
    Table,
    Id,
    AssetId,
    Timestamp,
    PriceUsd,
    MarketCapUsd,
    Source,
    CreatedAt,
    Rank,
    CirculatingSupply,
    TotalSupply,
    MaxSupply,
    BetaValue,
    AthPrice,
    AthDate,
    PercentFromPriceAth,
}

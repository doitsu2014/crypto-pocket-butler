use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AssetPrices::Table)
                    .if_not_exists()
                    .col(uuid(AssetPrices::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AssetPrices::AssetId).not_null())
                    .col(timestamp_with_time_zone(AssetPrices::Timestamp).not_null()) // Time of price snapshot
                    .col(decimal(AssetPrices::PriceUsd).not_null()) // Spot price in USD
                    .col(decimal_null(AssetPrices::Volume24hUsd)) // 24-hour trading volume in USD
                    .col(decimal_null(AssetPrices::MarketCapUsd)) // Market capitalization in USD
                    .col(decimal_null(AssetPrices::ChangePercent24h)) // 24-hour price change percentage
                    .col(string(AssetPrices::Source).not_null()) // Data source: e.g., "coingecko", "coinmarketcap", "exchange"
                    .col(timestamp_with_time_zone(AssetPrices::CreatedAt).default(Expr::current_timestamp()).not_null())
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

        // Create index on asset_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_asset_id")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::AssetId)
                    .to_owned(),
            )
            .await?;

        // Create index on timestamp for time-series queries
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_timestamp")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::Timestamp)
                    .to_owned(),
            )
            .await?;

        // Create composite index for efficient time-series queries per asset
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

        // Create unique index to prevent duplicate price entries for same asset/timestamp/source
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
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AssetPrices::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AssetPrices {
    Table,
    Id,
    AssetId,
    Timestamp,
    PriceUsd,
    Volume24hUsd,
    MarketCapUsd,
    ChangePercent24h,
    Source,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}

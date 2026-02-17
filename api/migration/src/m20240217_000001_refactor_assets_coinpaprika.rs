use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add missing fields to asset_prices table to consolidate with asset_rankings
        // This allows us to store all CoinPaprika data in one place per timestamp
        
        manager
            .alter_table(
                Table::alter()
                    .table(AssetPrices::Table)
                    // Add rank field from rankings
                    .add_column(integer_null(AssetPrices::Rank))
                    // Add supply information from CoinPaprika
                    .add_column(decimal_null(AssetPrices::CirculatingSupply))
                    .add_column(decimal_null(AssetPrices::TotalSupply))
                    .add_column(decimal_null(AssetPrices::MaxSupply))
                    // Add beta value
                    .add_column(decimal_null(AssetPrices::BetaValue))
                    // Add additional percent change fields
                    .add_column(decimal_null(AssetPrices::PercentChange1h))
                    .add_column(decimal_null(AssetPrices::PercentChange7d))
                    .add_column(decimal_null(AssetPrices::PercentChange30d))
                    // Add ATH (All-Time High) information
                    .add_column(decimal_null(AssetPrices::AthPrice))
                    .add_column(timestamp_with_time_zone_null(AssetPrices::AthDate))
                    .add_column(decimal_null(AssetPrices::PercentFromPriceAth))
                    .to_owned(),
            )
            .await?;

        // Create index on rank for efficient top-N queries
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_rank")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::Rank)
                    .to_owned(),
            )
            .await?;

        // Create composite index for date-based rank queries
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_timestamp_rank")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::Timestamp)
                    .col(AssetPrices::Rank)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the added indices
        manager
            .drop_index(
                Index::drop()
                    .name("idx_asset_prices_timestamp_rank")
                    .table(AssetPrices::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_asset_prices_rank")
                    .table(AssetPrices::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the added columns
        manager
            .alter_table(
                Table::alter()
                    .table(AssetPrices::Table)
                    .drop_column(AssetPrices::Rank)
                    .drop_column(AssetPrices::CirculatingSupply)
                    .drop_column(AssetPrices::TotalSupply)
                    .drop_column(AssetPrices::MaxSupply)
                    .drop_column(AssetPrices::BetaValue)
                    .drop_column(AssetPrices::PercentChange1h)
                    .drop_column(AssetPrices::PercentChange7d)
                    .drop_column(AssetPrices::PercentChange30d)
                    .drop_column(AssetPrices::AthPrice)
                    .drop_column(AssetPrices::AthDate)
                    .drop_column(AssetPrices::PercentFromPriceAth)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum AssetPrices {
    Table,
    Rank,
    CirculatingSupply,
    TotalSupply,
    MaxSupply,
    BetaValue,
    PercentChange1h,
    PercentChange7d,
    PercentChange30d,
    AthPrice,
    AthDate,
    PercentFromPriceAth,
    Timestamp,
}

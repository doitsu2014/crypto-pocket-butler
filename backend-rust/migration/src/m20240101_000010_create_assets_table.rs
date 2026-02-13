use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Assets::Table)
                    .if_not_exists()
                    .col(uuid(Assets::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(string(Assets::Symbol).not_null()) // e.g., "BTC", "ETH", "USDT"
                    .col(string(Assets::Name).not_null()) // e.g., "Bitcoin", "Ethereum"
                    .col(string(Assets::AssetType).not_null()) // e.g., "cryptocurrency", "token", "stablecoin"
                    .col(string_null(Assets::CoingeckoId)) // CoinGecko API ID for price data
                    .col(string_null(Assets::CoinmarketcapId)) // CoinMarketCap ID
                    .col(string_null(Assets::LogoUrl)) // URL to asset logo/icon
                    .col(text_null(Assets::Description)) // Asset description
                    .col(integer_null(Assets::Decimals)) // Token decimals (e.g., 18 for most ERC20)
                    .col(boolean(Assets::IsActive).default(true).not_null()) // Whether asset is actively tracked
                    .col(timestamp_with_time_zone(Assets::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Assets::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .to_owned(),
            )
            .await?;

        // Create unique index on symbol for fast lookups and prevent duplicates
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_symbol")
                    .table(Assets::Table)
                    .col(Assets::Symbol)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create index on asset_type for filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_asset_type")
                    .table(Assets::Table)
                    .col(Assets::AssetType)
                    .to_owned(),
            )
            .await?;

        // Create index on coingecko_id for API integrations
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_coingecko_id")
                    .table(Assets::Table)
                    .col(Assets::CoingeckoId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
    CoingeckoId,
    CoinmarketcapId,
    LogoUrl,
    Description,
    Decimals,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

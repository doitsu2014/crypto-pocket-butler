use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create assets table
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
            .await?;

        // Create asset_contracts table
        manager
            .create_table(
                Table::create()
                    .table(AssetContracts::Table)
                    .if_not_exists()
                    .col(uuid(AssetContracts::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AssetContracts::AssetId).not_null())
                    .col(string(AssetContracts::Chain).not_null()) // e.g., "ethereum", "bsc", "polygon", "arbitrum"
                    .col(string(AssetContracts::ContractAddress).not_null()) // Contract address on the chain
                    .col(string_null(AssetContracts::TokenStandard)) // e.g., "ERC20", "BEP20", "ERC721"
                    .col(integer_null(AssetContracts::Decimals)) // Token decimals (can override asset default)
                    .col(boolean(AssetContracts::IsVerified).default(false).not_null()) // Whether contract is verified
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

        // Create index on asset_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_asset_id")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::AssetId)
                    .to_owned(),
            )
            .await?;

        // Create index on chain for filtering by blockchain
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_chain")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::Chain)
                    .to_owned(),
            )
            .await?;

        // Create unique index to prevent duplicate contract addresses per chain
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

        // Create asset_prices table
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
            .await?;

        // Create asset_rankings table
        manager
            .create_table(
                Table::create()
                    .table(AssetRankings::Table)
                    .if_not_exists()
                    .col(uuid(AssetRankings::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AssetRankings::AssetId).not_null())
                    .col(date(AssetRankings::SnapshotDate).not_null()) // Date of ranking snapshot
                    .col(integer(AssetRankings::Rank).not_null()) // Market cap rank (1-100+)
                    .col(decimal(AssetRankings::MarketCapUsd).not_null()) // Market cap at snapshot time
                    .col(decimal(AssetRankings::PriceUsd).not_null()) // Price at snapshot time
                    .col(decimal_null(AssetRankings::Volume24hUsd)) // 24-hour volume at snapshot time
                    .col(decimal_null(AssetRankings::ChangePercent24h)) // 24-hour change at snapshot time
                    .col(decimal_null(AssetRankings::Dominance)) // Market dominance percentage
                    .col(string(AssetRankings::Source).not_null()) // Data source: e.g., "coingecko", "coinmarketcap"
                    .col(timestamp_with_time_zone(AssetRankings::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_rankings_asset_id")
                            .from(AssetRankings::Table, AssetRankings::AssetId)
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
                    .name("idx_asset_rankings_asset_id")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::AssetId)
                    .to_owned(),
            )
            .await?;

        // Create index on snapshot_date for time-series queries
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_rankings_snapshot_date")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::SnapshotDate)
                    .to_owned(),
            )
            .await?;

        // Create index on rank for top-N queries
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_rankings_rank")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::Rank)
                    .to_owned(),
            )
            .await?;

        // Create composite index for efficient queries of top assets on specific dates
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_rankings_date_rank")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::SnapshotDate)
                    .col(AssetRankings::Rank)
                    .to_owned(),
            )
            .await?;

        // Create unique index to prevent duplicate rankings for same asset/date/source
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_rankings_unique")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::AssetId)
                    .col(AssetRankings::SnapshotDate)
                    .col(AssetRankings::Source)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AssetRankings::Table).to_owned())
            .await?;
        
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
    CoingeckoId,
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
    Volume24hUsd,
    MarketCapUsd,
    ChangePercent24h,
    Source,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AssetRankings {
    Table,
    Id,
    AssetId,
    SnapshotDate,
    Rank,
    MarketCapUsd,
    PriceUsd,
    Volume24hUsd,
    ChangePercent24h,
    Dominance,
    Source,
    CreatedAt,
}

use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
            .await
    }
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

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}

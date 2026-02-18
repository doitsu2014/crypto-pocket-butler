use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the asset_rankings table as it's no longer needed
        // The asset_prices table now contains all necessary ranking and market data
        // due to the previous migration that added rank and other fields to asset_prices
        manager
            .drop_table(Table::drop().table(AssetRankings::Table).to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Recreate asset_rankings table if we need to rollback
        manager
            .create_table(
                Table::create()
                    .table(AssetRankings::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AssetRankings::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(AssetRankings::AssetId).uuid().not_null())
                    .col(ColumnDef::new(AssetRankings::SnapshotDate).date().not_null())
                    .col(ColumnDef::new(AssetRankings::Rank).integer().not_null())
                    .col(ColumnDef::new(AssetRankings::MarketCapUsd).decimal().not_null())
                    .col(ColumnDef::new(AssetRankings::PriceUsd).decimal().not_null())
                    .col(ColumnDef::new(AssetRankings::Volume24hUsd).decimal())
                    .col(ColumnDef::new(AssetRankings::ChangePercent24h).decimal())
                    .col(ColumnDef::new(AssetRankings::Dominance).decimal())
                    .col(ColumnDef::new(AssetRankings::Source).string().not_null())
                    .col(
                        ColumnDef::new(AssetRankings::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp())
                    )
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

        // Recreate indices
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_rankings_asset_id")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::AssetId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_rankings_snapshot_date")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::SnapshotDate)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_rankings_rank")
                    .table(AssetRankings::Table)
                    .col(AssetRankings::Rank)
                    .to_owned(),
            )
            .await?;

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

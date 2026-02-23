use sea_orm_migration::prelude::*;

/// Migration: add a `date` column to `asset_prices` for efficient daily time-series queries.
///
/// The column is derived from the existing `timestamp` field (UTC date part only) so that
/// callers can efficiently group or filter price records by calendar date without truncating
/// a full timestamp on every query.
///
/// Indexes added:
/// * `idx_asset_prices_date`             – single-column index on `date`
/// * `idx_asset_prices_asset_id_date`    – composite index on `(asset_id, date)` for per-asset
///                                         daily lookups (replaces a full timestamp scan)
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add the `date` column with a server-side default derived from `timestamp`.
        manager
            .alter_table(
                Table::alter()
                    .table(AssetPrices::Table)
                    .add_column(
                        ColumnDef::new(AssetPrices::Date)
                            .date()
                            .not_null()
                            .default(Expr::current_date()),
                    )
                    .to_owned(),
            )
            .await?;

        // Back-fill existing rows so that `date` mirrors `timestamp::date`.
        manager
            .get_connection()
            .execute_unprepared("UPDATE asset_prices SET date = (timestamp AT TIME ZONE 'UTC')::date")
            .await?;

        // Index: single-column on `date`
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_date")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::Date)
                    .to_owned(),
            )
            .await?;

        // Index: composite `(asset_id, date)` for per-asset daily lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_prices_asset_id_date")
                    .table(AssetPrices::Table)
                    .col(AssetPrices::AssetId)
                    .col(AssetPrices::Date)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_asset_prices_asset_id_date")
                    .table(AssetPrices::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_asset_prices_date")
                    .table(AssetPrices::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(AssetPrices::Table)
                    .drop_column(AssetPrices::Date)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum AssetPrices {
    Table,
    AssetId,
    Date,
}

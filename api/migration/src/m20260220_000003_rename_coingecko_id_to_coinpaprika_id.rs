use sea_orm_migration::prelude::*;

/// Rename `assets.coingecko_id` → `assets.coinpaprika_id`.
///
/// The column was originally named for CoinGecko but has always stored CoinPaprika IDs
/// since the system switched to CoinPaprika as its market-data source.  The new name
/// accurately reflects the stored value and removes the misleading "coingecko" reference.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename the index before renaming the column so we don't leave a stale index name.
        manager
            .drop_index(
                Index::drop()
                    .name("idx_assets_coingecko_id")
                    .table(Assets::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE assets RENAME COLUMN coingecko_id TO coinpaprika_id",
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_assets_coinpaprika_id")
                    .table(Assets::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE assets RENAME COLUMN coinpaprika_id TO coingecko_id",
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assets_coingecko_id")
                    .table(Assets::Table)
                    .col(Assets::CoingeckoId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    CoinpaprikaId,
    CoingeckoId, // used in down() only
}

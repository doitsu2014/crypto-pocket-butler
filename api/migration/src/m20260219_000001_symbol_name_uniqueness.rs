use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the old unique index on symbol only
        manager
            .drop_index(
                Index::drop()
                    .name("idx_assets_symbol")
                    .table(Assets::Table)
                    .to_owned(),
            )
            .await?;

        // Create a new unique index on (symbol, name) combination
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the composite unique index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_assets_symbol_name_unique")
                    .table(Assets::Table)
                    .to_owned(),
            )
            .await?;

        // Restore the old unique index on symbol only
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

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Symbol,
    Name,
}

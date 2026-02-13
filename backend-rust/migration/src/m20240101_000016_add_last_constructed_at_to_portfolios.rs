use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Portfolios::Table)
                    .add_column(timestamp_with_time_zone_null(Portfolios::LastConstructedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Portfolios::Table)
                    .drop_column(Portfolios::LastConstructedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    LastConstructedAt,
}

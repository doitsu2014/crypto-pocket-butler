use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Portfolios::Table)
                    .if_not_exists()
                    .col(uuid(Portfolios::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Portfolios::UserId).not_null())
                    .col(string(Portfolios::Name).not_null())
                    .col(text_null(Portfolios::Description))
                    .col(boolean(Portfolios::IsDefault).default(false).not_null())
                    .col(timestamp_with_time_zone(Portfolios::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Portfolios::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolios_user_id")
                            .from(Portfolios::Table, Portfolios::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_portfolios_user_id")
                    .table(Portfolios::Table)
                    .col(Portfolios::UserId)
                    .to_owned(),
            )
            .await?;

        // Create unique index to ensure only one default portfolio per user
        // Using raw SQL for partial unique index (WHERE is_default = true)
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_portfolios_user_id_is_default \
                 ON portfolios (user_id) WHERE is_default = true"
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Portfolios::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
    UserId,
    Name,
    Description,
    IsDefault,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

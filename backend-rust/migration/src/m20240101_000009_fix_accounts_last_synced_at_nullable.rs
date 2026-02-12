use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Alter the last_synced_at column to be nullable
        // The original migration incorrectly used timestamp_with_time_zone() which creates NOT NULL
        // Should use timestamp_with_time_zone_null() for nullable timestamps
        manager
            .alter_table(
                Table::alter()
                    .table(Accounts::Table)
                    .modify_column(
                        ColumnDef::new(Accounts::LastSyncedAt)
                            .timestamp_with_time_zone()
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert to NOT NULL (original incorrect state)
        // Note: This will fail if there are NULL values in the column
        manager
            .alter_table(
                Table::alter()
                    .table(Accounts::Table)
                    .modify_column(
                        ColumnDef::new(Accounts::LastSyncedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    LastSyncedAt,
}

use sea_orm_migration::{prelude::*, schema::uuid_null};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add allocation_id column to snapshots table as optional reference
        manager
            .alter_table(
                Table::alter()
                    .table(Snapshots::Table)
                    .add_column(uuid_null(Snapshots::AllocationId))
                    .to_owned(),
            )
            .await?;

        // Add foreign key to portfolio_allocations
        manager
            .alter_table(
                Table::alter()
                    .table(Snapshots::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_snapshots_allocation_id")
                            .from_tbl(Snapshots::Table)
                            .from_col(Snapshots::AllocationId)
                            .to_tbl(PortfolioAllocations::Table)
                            .to_col(PortfolioAllocations::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on allocation_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_allocation_id")
                    .table(Snapshots::Table)
                    .col(Snapshots::AllocationId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop foreign key and index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_snapshots_allocation_id")
                    .table(Snapshots::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_snapshots_allocation_id")
                    .table(Snapshots::Table)
                    .to_owned(),
            )
            .await?;

        // Drop column
        manager
            .alter_table(
                Table::alter()
                    .table(Snapshots::Table)
                    .drop_column(Snapshots::AllocationId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Snapshots {
    Table,
    AllocationId,
}

#[derive(DeriveIden)]
enum PortfolioAllocations {
    Table,
    Id,
}

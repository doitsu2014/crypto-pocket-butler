use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add target_allocation JSON column
        manager
            .alter_table(
                Table::alter()
                    .table(Portfolios::Table)
                    .add_column(ColumnDef::new(Portfolios::TargetAllocation).json_binary().null())
                    .to_owned(),
            )
            .await?;

        // Add guardrails JSON column
        manager
            .alter_table(
                Table::alter()
                    .table(Portfolios::Table)
                    .add_column(ColumnDef::new(Portfolios::Guardrails).json_binary().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Portfolios::Table)
                    .drop_column(Portfolios::Guardrails)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Portfolios::Table)
                    .drop_column(Portfolios::TargetAllocation)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    TargetAllocation,
    Guardrails,
}

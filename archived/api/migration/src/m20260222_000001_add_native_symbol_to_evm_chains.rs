use sea_orm::{sea_query::Values, DbBackend, Statement};
use sea_orm_migration::{prelude::*, schema::*};

/// Adds a `native_symbol` column to the `evm_chains` table so that each chain
/// carries its native token symbol (e.g. "ETH", "BNB", "HYPE") directly in the DB.
///
/// This removes the need for a hardcoded enum in Rust code: every chain seeded or
/// added at runtime will be fully described by its DB row.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add the column with a safe default so existing rows are not null
        manager
            .alter_table(
                Table::alter()
                    .table(EvmChains::Table)
                    .add_column(
                        string(EvmChains::NativeSymbol)
                            .not_null()
                            .default("ETH"),
                    )
                    .to_owned(),
            )
            .await?;

        // Correct the native symbols for chains whose symbol differs from "ETH"
        let db = manager.get_connection();
        for (chain_id, native_symbol) in [("bsc", "BNB"), ("hyper_liquid", "HYPE"), ("mantle", "MNT")] {
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE evm_chains SET native_symbol = $1 WHERE chain_id = $2",
                Values(vec![native_symbol.into(), chain_id.into()]),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(EvmChains::Table)
                    .drop_column(EvmChains::NativeSymbol)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum EvmChains {
    Table,
    NativeSymbol,
}

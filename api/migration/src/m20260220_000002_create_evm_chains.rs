use sea_orm_migration::{prelude::*, schema::*};
use sea_orm::{Statement, DbBackend, sea_query::Values};

/// Creates the `evm_chains` table and seeds it with default EVM chain configurations.
///
/// Each row represents a supported EVM-compatible chain with a configurable RPC URL.
/// Chains can be managed at runtime via `/api/v1/evm-chains`.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Default chain seeds: (chain_id, name, rpc_url)
const SEED_CHAINS: &[(&str, &str, &str)] = &[
    ("ethereum",  "Ethereum",        "https://eth.llamarpc.com"),
    ("arbitrum",  "Arbitrum",        "https://arbitrum.llamarpc.com"),
    ("optimism",  "Optimism",        "https://optimism.llamarpc.com"),
    ("base",      "Base",            "https://base.llamarpc.com"),
    ("bsc",       "BNB Smart Chain", "https://bsc-dataseed.bnbchain.org"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EvmChains::Table)
                    .if_not_exists()
                    .col(
                        uuid(EvmChains::Id)
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(string(EvmChains::ChainId).not_null())
                    .col(string(EvmChains::Name).not_null())
                    .col(string(EvmChains::RpcUrl).not_null())
                    .col(boolean(EvmChains::IsActive).default(true).not_null())
                    .col(
                        timestamp_with_time_zone(EvmChains::CreatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        timestamp_with_time_zone(EvmChains::UpdatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_evm_chains_chain_id_unique")
                    .table(EvmChains::Table)
                    .col(EvmChains::ChainId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_evm_chains_is_active")
                    .table(EvmChains::Table)
                    .col(EvmChains::IsActive)
                    .to_owned(),
            )
            .await?;

        // Seed default chains
        let db = manager.get_connection();
        for (chain_id, name, rpc_url) in SEED_CHAINS {
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO evm_chains (chain_id, name, rpc_url, is_active) \
                 VALUES ($1, $2, $3, true) \
                 ON CONFLICT (chain_id) DO NOTHING",
                Values(vec![
                    (*chain_id).into(),
                    (*name).into(),
                    (*rpc_url).into(),
                ]),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EvmChains::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EvmChains {
    Table,
    Id,
    ChainId,
    Name,
    RpcUrl,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

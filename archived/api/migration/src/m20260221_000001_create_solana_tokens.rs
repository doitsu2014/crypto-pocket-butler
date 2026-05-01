use sea_orm_migration::{prelude::*, schema::*};
use sea_orm::{Statement, DbBackend, sea_query::Values};

/// Creates the `solana_tokens` table and seeds it with well-known SPL token mint addresses.
///
/// The table stores the DB-driven list of SPL token mint addresses that the
/// Solana connector checks during account sync. Manageable at runtime via
/// `/api/v1/solana-tokens`.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Raw seed data: (symbol, mint_address)
const SEED_TOKENS: &[(&str, &str)] = &[
    // Stablecoins
    ("USDC",    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
    ("USDT",    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
    // Liquid staking / wrapped
    ("MSOL",    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"),
    ("STSOL",   "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj"),
    ("JITOSOL", "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn"),
    // DeFi / blue chips
    ("RAY",     "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"),
    ("JUP",     "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"),
    ("PYTH",    "HZ1JovNiVvGrk8Zas8vbMGMBBHHNzFn2Gb8E9Z4vNxBL"),
    ("JTO",     "jtojtomepa8beP8AuQc6eXt5FriJwfFMwjx2v2f9mUL"),
    // Meme tokens
    ("BONK",    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
    ("WIF",     "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── 1. Create solana_tokens table ─────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(SolanaTokens::Table)
                    .if_not_exists()
                    .col(
                        uuid(SolanaTokens::Id)
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(string(SolanaTokens::Symbol).not_null())
                    .col(string(SolanaTokens::MintAddress).not_null())
                    .col(boolean(SolanaTokens::IsActive).default(true).not_null())
                    .col(
                        timestamp_with_time_zone(SolanaTokens::CreatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        timestamp_with_time_zone(SolanaTokens::UpdatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_solana_tokens_mint_address_unique")
                    .table(SolanaTokens::Table)
                    .col(SolanaTokens::MintAddress)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_solana_tokens_is_active")
                    .table(SolanaTokens::Table)
                    .col(SolanaTokens::IsActive)
                    .to_owned(),
            )
            .await?;

        // ── 2. Seed solana_tokens with well-known SPL mint addresses ──────────
        let db = manager.get_connection();
        for (symbol, mint_address) in SEED_TOKENS {
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO solana_tokens (symbol, mint_address, is_active) \
                 VALUES ($1, $2, true) \
                 ON CONFLICT (mint_address) DO NOTHING",
                Values(vec![
                    (*symbol).into(),
                    (*mint_address).into(),
                ]),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SolanaTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SolanaTokens {
    Table,
    Id,
    Symbol,
    MintAddress,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

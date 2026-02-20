use sea_orm_migration::{prelude::*, schema::*};
use sea_orm::{Statement, DbBackend, sea_query::Values};

/// Consolidates all 2026-02-20 schema changes into one migration:
///
/// 1. Creates the `evm_tokens` table — stores the DB-driven list of ERC-20 token addresses the
///    EVM connector checks during account sync.  Manageable at runtime via `/api/v1/evm-tokens`.
///
/// 2. Seeds `evm_tokens` with a curated set of well-known tokens across all supported EVM chains
///    (Ethereum 22, Arbitrum 16, Optimism 16, Base 9, BSC 12).
///
/// 3. Renames `assets.coingecko_id` → `assets.coinpaprika_id` — the column has always stored
///    CoinPaprika IDs; the name previously referenced the wrong data source.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Raw seed data: (chain, symbol, contract_address)
const SEED_TOKENS: &[(&str, &str, &str)] = &[
    // ── Ethereum ─────────────────────────────────────────────────────────────
    ("ethereum", "USDT",   "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
    ("ethereum", "USDC",   "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
    ("ethereum", "DAI",    "0x6B175474E89094C44Da98b954EedeAC495271d0F"),
    ("ethereum", "FRAX",   "0x853d955aCEf822Db058eb8505911ED77F175b99e"),
    ("ethereum", "LUSD",   "0x5f98805A4E8be255a32880FDeC7F6728C6568bA0"),
    ("ethereum", "WETH",   "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
    ("ethereum", "WBTC",   "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"),
    ("ethereum", "STETH",  "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84"),
    ("ethereum", "WSTETH", "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0"),
    ("ethereum", "RETH",   "0xae78736Cd615f374D3085123A210448E74Fc6393"),
    ("ethereum", "CBETH",  "0xBe9895146f7AF43049ca1c1AE358B0541Ea49704"),
    ("ethereum", "LINK",   "0x514910771AF9Ca656af840dff83E8264EcF986CA"),
    ("ethereum", "UNI",    "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
    ("ethereum", "AAVE",   "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9"),
    ("ethereum", "MKR",    "0x9f8F72aA9304c8B593d555F12eF6589cC3A579A2"),
    ("ethereum", "COMP",   "0xc00e94Cb662C3520282E6f5717214004A7f26888"),
    ("ethereum", "CRV",    "0xD533a949740bb3306d119CC777fa900bA034cd52"),
    ("ethereum", "LDO",    "0x5A98FcBEA516Cf06857215779Fd812CA3beF1B32"),
    ("ethereum", "SNX",    "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F"),
    ("ethereum", "BAL",    "0xba100000625a3754423978a60c9317c58a424e3D"),
    ("ethereum", "1INCH",  "0x111111111117dC0aa78b770fA6A738034120C302"),
    ("ethereum", "ENS",    "0xC18360217D8F7Ab5e7c516566761Ea12Ce7F9D72"),
    // ── Arbitrum ─────────────────────────────────────────────────────────────
    ("arbitrum", "USDT",   "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"),
    ("arbitrum", "USDC",   "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),
    ("arbitrum", "DAI",    "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
    ("arbitrum", "FRAX",   "0x17FC002b466eEc40DaE837Fc4bE5c67993ddBd6F"),
    ("arbitrum", "LUSD",   "0x93b346b6BC2548dA6A1E7d98E9a421B42541425b"),
    ("arbitrum", "WETH",   "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"),
    ("arbitrum", "WBTC",   "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f"),
    ("arbitrum", "WSTETH", "0x5979D7b546E38E414F7E9822514be443A4800529"),
    ("arbitrum", "RETH",   "0xEC70Dcb4A1EFa46b8F2D97C310C9c4790ba5ffA8"),
    ("arbitrum", "LINK",   "0xf97f4df75117a78c1A5a0DBb814Af92458539FB4"),
    ("arbitrum", "UNI",    "0xFa7F8980b0f1E64A2062791cc3b0871572f1F7f0"),
    ("arbitrum", "AAVE",   "0xba5DdD1f9d7F570dc94a51479a000E3BCE967196"),
    ("arbitrum", "CRV",    "0x11cDb42B0EB46D95f990BeDD4695A6e3fA034978"),
    ("arbitrum", "GMX",    "0xfc5A1A6EB076a2C7aD06eD22C90d7E710E35ad0a"),
    ("arbitrum", "ARB",    "0x912CE59144191C1204E64559FE8253a0e49E6548"),
    // ── Optimism ─────────────────────────────────────────────────────────────
    ("optimism", "USDT",   "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58"),
    ("optimism", "USDC",   "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85"),
    ("optimism", "DAI",    "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
    ("optimism", "FRAX",   "0x2E3D870790dC77A83DD1d18184Acc7439A53f475"),
    ("optimism", "LUSD",   "0xc40F949F8a4e094D1b49a23ea9241D289B7b2819"),
    ("optimism", "WETH",   "0x4200000000000000000000000000000000000006"),
    ("optimism", "WBTC",   "0x68f180fcCe6836688e9084f035309E29Bf0A2095"),
    ("optimism", "WSTETH", "0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb"),
    ("optimism", "RETH",   "0x9Bcef72be871e61ED4fBbc7630889beE758eb81D"),
    ("optimism", "LINK",   "0x350a791Bfc2C21F9Ed5d10980Dad2e2638ffa7f6"),
    ("optimism", "UNI",    "0x6fd9d7AD17242c41f7131d257212c54A0e816691"),
    ("optimism", "AAVE",   "0x76FB31fb4af56892A25e32cFC43De717950c9278"),
    ("optimism", "CRV",    "0x0994206dfE8De6Ec6920FF4D779B0d950605Fb53"),
    ("optimism", "OP",     "0x4200000000000000000000000000000000000042"),
    ("optimism", "SNX",    "0x8700dAec35aF8Ff88c16BdF0418774CB3D7599B4"),
    // ── Base ─────────────────────────────────────────────────────────────────
    ("base",     "USDC",   "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
    ("base",     "DAI",    "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb"),
    ("base",     "USDT",   "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2"),
    ("base",     "WETH",   "0x4200000000000000000000000000000000000006"),
    ("base",     "WBTC",   "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf"),
    ("base",     "WSTETH", "0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452"),
    ("base",     "CBETH",  "0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22"),
    ("base",     "RETH",   "0xB6fe221Fe9EeF5aBa221c348bA20A1Bf5e73624c"),
    ("base",     "AERO",   "0x940181a94A35A4569E4529A3CDfB74e38FD98631"),
    // ── BNB Smart Chain ──────────────────────────────────────────────────────
    ("bsc",      "USDT",   "0x55d398326f99059fF775485246999027B3197955"),
    ("bsc",      "USDC",   "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d"),
    ("bsc",      "DAI",    "0x1AF3F329e8BE154074D8769D1FFa4eE058B1DBc3"),
    ("bsc",      "BUSD",   "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"),
    ("bsc",      "FDUSD",  "0xc5f0f7b66764F6ec8C8Dff7BA683102295E16409"),
    ("bsc",      "WBNB",   "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"),
    ("bsc",      "BTCB",   "0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c"),
    ("bsc",      "ETH",    "0x2170Ed0880ac9A755fd29B2688956BD959F933F8"),
    ("bsc",      "CAKE",   "0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82"),
    ("bsc",      "XVS",    "0xcF6BB5389c92Bdda8a3747Ddb454cB7a64626C63"),
    ("bsc",      "ALPACA", "0x8F0528cE5eF7B51152A59745bEfDD91D97091d2F"),
    ("bsc",      "LINK",   "0xF8A0BF9cF54Bb92F17374d9e9A321E6a111a51bD"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── 1. Create evm_tokens table ────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(EvmTokens::Table)
                    .if_not_exists()
                    .col(
                        uuid(EvmTokens::Id)
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(string(EvmTokens::Chain).not_null())
                    .col(string(EvmTokens::Symbol).not_null())
                    .col(string(EvmTokens::ContractAddress).not_null())
                    .col(boolean(EvmTokens::IsActive).default(true).not_null())
                    .col(
                        timestamp_with_time_zone(EvmTokens::CreatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        timestamp_with_time_zone(EvmTokens::UpdatedAt)
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_evm_tokens_chain_address_unique")
                    .table(EvmTokens::Table)
                    .col(EvmTokens::Chain)
                    .col(EvmTokens::ContractAddress)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_evm_tokens_chain")
                    .table(EvmTokens::Table)
                    .col(EvmTokens::Chain)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_evm_tokens_is_active")
                    .table(EvmTokens::Table)
                    .col(EvmTokens::IsActive)
                    .to_owned(),
            )
            .await?;

        // ── 2. Seed evm_tokens with well-known ERC-20 addresses ───────────────
        let db = manager.get_connection();
        for (chain, symbol, address) in SEED_TOKENS {
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO evm_tokens (chain, symbol, contract_address, is_active) \
                 VALUES ($1, $2, $3, true) \
                 ON CONFLICT (chain, contract_address) DO NOTHING",
                Values(vec![
                    (*chain).into(),
                    (*symbol).into(),
                    (*address).into(),
                ]),
            ))
            .await?;
        }

        // ── 3. Rename assets.coingecko_id → assets.coinpaprika_id ────────────
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
        // ── Undo 3: rename coinpaprika_id back to coingecko_id ────────────────
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

        // ── Undo 1+2: drop evm_tokens (seed rows go with the table) ──────────
        manager
            .drop_table(Table::drop().table(EvmTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EvmTokens {
    Table,
    Id,
    Chain,
    Symbol,
    ContractAddress,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    CoinpaprikaId,
    CoingeckoId, // used in down() only
}

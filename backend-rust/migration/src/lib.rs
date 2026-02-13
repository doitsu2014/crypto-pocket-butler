pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_users_table;
mod m20240101_000002_create_accounts_table;
mod m20240101_000003_create_portfolios_table;
mod m20240101_000004_create_portfolio_accounts_table;
mod m20240101_000005_create_snapshots_table;
mod m20240101_000006_add_holdings_to_accounts;
mod m20240101_000007_create_recommendations_table;
mod m20240101_000008_add_settings_to_portfolios;
mod m20240101_000009_fix_accounts_last_synced_at_nullable;
mod m20240101_000010_create_assets_table;
mod m20240101_000011_create_asset_contracts_table;
mod m20240101_000012_create_asset_prices_table;
mod m20240101_000013_create_asset_rankings_table;
mod m20240101_000014_add_enabled_chains_to_accounts;
mod m20240101_000015_create_portfolio_allocations_table;
mod m20240101_000016_add_last_constructed_at_to_portfolios;
mod m20240101_000017_add_unique_portfolio_id_to_allocations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_users_table::Migration),
            Box::new(m20240101_000002_create_accounts_table::Migration),
            Box::new(m20240101_000003_create_portfolios_table::Migration),
            Box::new(m20240101_000004_create_portfolio_accounts_table::Migration),
            Box::new(m20240101_000005_create_snapshots_table::Migration),
            Box::new(m20240101_000006_add_holdings_to_accounts::Migration),
            Box::new(m20240101_000007_create_recommendations_table::Migration),
            Box::new(m20240101_000008_add_settings_to_portfolios::Migration),
            Box::new(m20240101_000009_fix_accounts_last_synced_at_nullable::Migration),
            Box::new(m20240101_000010_create_assets_table::Migration),
            Box::new(m20240101_000011_create_asset_contracts_table::Migration),
            Box::new(m20240101_000012_create_asset_prices_table::Migration),
            Box::new(m20240101_000013_create_asset_rankings_table::Migration),
            Box::new(m20240101_000014_add_enabled_chains_to_accounts::Migration),
            Box::new(m20240101_000015_create_portfolio_allocations_table::Migration),
            Box::new(m20240101_000016_add_last_constructed_at_to_portfolios::Migration),
            Box::new(m20240101_000017_add_unique_portfolio_id_to_allocations::Migration),
        ]
    }
}

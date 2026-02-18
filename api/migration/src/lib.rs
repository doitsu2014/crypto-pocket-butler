pub use sea_orm_migration::prelude::*;

// Consolidated migrations for release
mod m20240101_000001_create_core_user_account_system;
mod m20240101_000002_create_portfolio_management_system;
mod m20240101_000003_create_assets_system;
mod m20240101_000004_create_portfolio_allocations;
mod m20240101_000005_create_snapshots_system;
mod m20240217_000001_refactor_assets_coinpaprika;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_core_user_account_system::Migration),
            Box::new(m20240101_000002_create_portfolio_management_system::Migration),
            Box::new(m20240101_000003_create_assets_system::Migration),
            Box::new(m20240101_000004_create_portfolio_allocations::Migration),
            Box::new(m20240101_000005_create_snapshots_system::Migration),
            Box::new(m20240217_000001_refactor_assets_coinpaprika::Migration),
        ]
    }
}

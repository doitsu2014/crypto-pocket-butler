pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_core_system;
mod m20240101_000002_create_assets_system;
mod m20260219_000002_normalize_holdings;
mod m20260220_000001_create_evm_tokens;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_core_system::Migration),
            Box::new(m20240101_000002_create_assets_system::Migration),
            Box::new(m20260219_000002_normalize_holdings::Migration),
            Box::new(m20260220_000001_create_evm_tokens::Migration),
        ]
    }
}

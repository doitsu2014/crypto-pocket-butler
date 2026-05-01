/// Infrastructure persistence layer
///
/// SeaORM-backed implementations of the domain repository traits.
///
/// ## Repository implementations
///
/// - [`AccountRepositoryImpl`] — `AccountRepository` backed by the `accounts` table
/// - [`PortfolioRepositoryImpl`] — `PortfolioRepository` backed by the `portfolios` table
/// - [`AssetRepositoryImpl`] — `AssetRepository` backed by the `assets` / `asset_prices` tables
/// - [`ChainRepositoryImpl`] — `ChainRepository` backed by the `evm_chains` table

pub mod entities;

pub mod account_repo;
pub mod asset_repo;
pub mod chain_repo;
pub mod portfolio_repo;

pub use account_repo::AccountRepositoryImpl;
pub use asset_repo::AssetRepositoryImpl;
pub use chain_repo::ChainRepositoryImpl;
pub use portfolio_repo::PortfolioRepositoryImpl;

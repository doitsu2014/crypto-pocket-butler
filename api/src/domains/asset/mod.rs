/// Asset Domain
///
/// Bounded Context: Asset definitions, pricing, and contract addresses.
/// Aggregate Root: Asset
///
/// This module defines:
/// - `Asset` aggregate root
/// - `AssetPrice` and `AssetContract` entities
/// - `AssetRepository` trait

pub mod aggregate;
pub mod entities;
pub mod repository;
pub mod service;

pub use aggregate::Asset;
pub use repository::AssetRepository;

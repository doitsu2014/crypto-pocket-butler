/// Portfolio Domain
///
/// Bounded Context: Portfolio management, target allocations, and snapshots.
/// Aggregate Root: Portfolio
///
/// This module defines:
/// - `Portfolio` aggregate root
/// - `TargetAllocation` and `Guardrails` value objects
/// - `PortfolioRepository` trait

pub mod aggregate;
pub mod value_objects;
pub mod entities;
pub mod repository;
pub mod service;

pub use aggregate::Portfolio;
pub use value_objects::{TargetAllocation, Guardrails};
pub use repository::PortfolioRepository;

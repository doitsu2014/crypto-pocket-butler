/// Use case modules for the application layer.
///
/// Each sub-module groups the use cases for a single bounded context.
/// HTTP handlers call into these use cases; they must not import
/// infrastructure entities or domain types directly.

pub mod account_usecases;
pub mod chain_usecases;
pub mod portfolio_usecases;
pub mod recommendation_usecases;
pub mod snapshot_usecases;

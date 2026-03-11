/// Infrastructure layer — persistence implementations and external service adapters
///
/// This module provides concrete implementations of the domain repository traits
/// using SeaORM for persistence and HTTP connectors for external services.
///
/// ## Sub-modules
///
/// - [`persistence`] — SeaORM-backed repository implementations
/// - [`external`]    — External API connectors (exchanges, price feeds, blockchains)
/// - [`cache`]       — In-memory caching layer

pub mod cache;
pub mod external;
pub mod persistence;

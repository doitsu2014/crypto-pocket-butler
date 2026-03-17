// Re-export axum-keycloak-auth for convenience
pub use axum_keycloak_auth;

// ─── DDD Architecture ────────────────────────────────────────────────────────

/// Domain layer — bounded contexts with aggregate roots, value objects,
/// entities, repository traits, and domain services.
pub mod domains;

/// Infrastructure layer — SeaORM persistence implementations and external
/// service adapters (connectors).
pub mod infrastructure;

/// Application layer — application services that orchestrate domain logic,
/// use cases that transport handlers call, and DTOs for the HTTP boundary.
pub mod application;

// ─── Legacy / existing modules (kept for backward compatibility) ─────────────

pub mod cache;
pub mod db;
pub mod helpers;
pub mod transport;

// Re-export migration for convenience
pub use migration;


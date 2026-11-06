//! Audit HTTP boundary: Axum router, DTOs, middleware.

pub mod dto;
pub mod export_stream;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod problem;
pub mod rate_limit;
pub mod routes;
pub mod server;
pub mod state;

pub use routes::router;
pub use state::AppState;

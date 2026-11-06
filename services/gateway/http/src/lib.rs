//! API Gateway HTTP boundary: Axum edge, security headers, rate limiting.

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

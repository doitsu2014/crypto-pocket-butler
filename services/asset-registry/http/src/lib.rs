//! Asset Registry HTTP boundary: Axum router, DTOs, middleware.

pub mod dto;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod problem;
pub mod routes;
pub mod server;
pub mod state;

pub use routes::router;
pub use state::AppState;

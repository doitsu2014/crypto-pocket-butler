//! Notification HTTP boundary: two Axum routers sharing one state.

pub mod dto;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod problem;
pub mod routes;
pub mod server;
pub mod state;

pub use routes::{api_router, worker_router};
pub use state::AppState;

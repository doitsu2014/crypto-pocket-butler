//! Identity HTTP boundary: Axum routers, DTOs, middleware, and the worker's
//! internal listener.

pub mod authorisation;
pub mod consent;
pub mod dto;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod problem;
pub mod routes;
pub mod server;
pub mod state;
pub mod worker;

pub use routes::router;
pub use state::AppState;

//! The gateway edge router.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::get;
use axum::Router;

use crate::handlers::{health, metrics, proxy, ready};
use crate::middleware::{client_ip, correlation_id, rate_limit, record_metrics, security_headers};
use crate::state::AppState;

/// Builds the router. Health, readiness, and metrics are local; everything else
/// goes through the deny-by-default gateway request path.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics))
        .fallback(proxy)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .layer(middleware::from_fn(client_ip))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .with_state(state)
}

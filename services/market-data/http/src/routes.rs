//! The Market Data router. Health, readiness, and metrics are public; every
//! data path needs a registered internal service token.

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::{fallback, get_fx, get_prices, health, metrics, ready, run_refresh};
use crate::middleware::{correlation_id, record_metrics, require_service, security_headers};
use crate::state::AppState;

/// Builds the router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let internal = Router::new()
        .route("/v1/prices", get(get_prices))
        .route("/v1/fx", get(get_fx))
        .route("/v1/internal/run/{jobName}", post(run_refresh))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ));

    Router::new()
        .merge(public)
        .merge(internal)
        .fallback(fallback)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .with_state(state)
}

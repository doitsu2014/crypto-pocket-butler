//! The Asset Registry router.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, patch, post, put};
use axum::Router;

use crate::handlers::{
    create_asset, create_token_contract, fallback, health, ignore_observation, metrics, ready,
    resolve_asset, resolve_observation, resolve_price, set_price_source_enabled,
    upsert_price_source,
};
use crate::middleware::{
    correlation_id, record_metrics, require_admin, require_auth, security_headers,
};
use crate::state::AppState;

/// Builds the router. Health, readiness, and metrics are public; registry reads
/// require a valid JWT; administration requires the admin role.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let protected = Router::new()
        .route("/v1/assets/resolve", get(resolve_asset))
        .route("/v1/assets/{asset_id}/price", get(resolve_price))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let admin = Router::new()
        .route("/v1/admin/assets", post(create_asset))
        .route("/v1/admin/token-contracts", post(create_token_contract))
        .route("/v1/admin/price-sources", put(upsert_price_source))
        .route(
            "/v1/admin/price-sources/{name}/enabled",
            patch(set_price_source_enabled),
        )
        .route(
            "/v1/admin/observations/{id}/resolve",
            post(resolve_observation),
        )
        .route(
            "/v1/admin/observations/{id}/ignore",
            post(ignore_observation),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(admin)
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .with_state(state)
}

//! The Reporting routers. Every authenticated path names its identity: the
//! forwarded Supabase JWT for the SPA surface, a service token for the internal
//! privacy and C11 paths. A path with no rule is not served.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::{
    fallback, get_brief, health, list_briefs, metrics, privacy_anonymise, privacy_export, ready,
    smoke,
};
use crate::middleware::{
    correlation_id, record_metrics, require_jwt, require_service, security_headers,
};
use crate::state::AppState;

/// The JSON request cap.
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;

/// Builds the API router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let authenticated = Router::new()
        .route("/v1/users/{userId}/briefs", get(list_briefs))
        .route("/v1/users/{userId}/briefs/{briefId}", get(get_brief))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    let internal = Router::new()
        .route("/v1/internal/privacy/export", get(privacy_export))
        .route("/v1/internal/privacy/anonymise", post(privacy_anonymise))
        .route("/v1/internal/smoke", get(smoke))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ));

    Router::new()
        .merge(public)
        .merge(authenticated)
        .merge(internal)
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .with_state(state)
}

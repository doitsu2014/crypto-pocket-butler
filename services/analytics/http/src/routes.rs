//! The two routers. The API serves C12 ingest, C1 flag/aggregate paths, erase,
//! and smoke on 8095. The worker serves the C11 run paths and probes on 8096.
//! Every path names its identity; a path with no rule is not served.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, patch, post};
use axum::Router;

use crate::handlers::{
    create_flag, delete_flag, erase, fallback, get_aggregates, get_flag, health, ingest_event,
    list_flags, metrics, ready, run_aggregate, run_retention, smoke, update_flag,
};
use crate::middleware::{
    correlation_id, record_metrics, require_admin, require_erase, require_run, require_smoke,
    require_user, security_headers,
};
use crate::state::AppState;

fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics))
}

fn finish(router: Router<AppState>, state: AppState) -> Router {
    router
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .with_state(state)
}

/// The API router.
pub fn api_router(state: AppState) -> Router {
    let user = Router::new()
        .route("/v1/events", post(ingest_event))
        .route("/v1/flags/{key}", get(get_flag))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_user));

    let admin = Router::new()
        .route("/v1/analytics/aggregates", get(get_aggregates))
        .route("/v1/admin/flags", get(list_flags).post(create_flag))
        .route(
            "/v1/admin/flags/{key}",
            patch(update_flag).delete(delete_flag),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));

    let smoke = Router::new()
        .route("/v1/internal/smoke", get(smoke))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_smoke));

    let erase = Router::new()
        .route("/v1/internal/privacy/erase", post(erase))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_erase));

    let router = public_routes()
        .merge(user)
        .merge(admin)
        .merge(smoke)
        .merge(erase);
    finish(router, state)
}

/// The worker router: C11 runs and probes.
pub fn worker_router(state: AppState) -> Router {
    let runs = Router::new()
        .route("/v1/internal/run/analytics-aggregate", post(run_aggregate))
        .route("/v1/internal/run/analytics-retention", post(run_retention))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_run));

    let router = public_routes().merge(runs);
    finish(router, state)
}

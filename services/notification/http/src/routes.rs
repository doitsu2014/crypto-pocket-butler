//! The two routers. The API serves C9 submit, C1 feed/preferences, erasure, and
//! smoke on 8099. The worker serves the C11 run paths and probes on 8100. Every
//! path names its identity; a path with no rule is not served.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::{
    anonymise, fallback, feed, get_preferences, health, metrics, put_preferences, ready,
    run_deliver, run_retention, smoke, submit_alert,
};
use crate::middleware::{
    correlation_id, record_metrics, require_anonymise, require_run, require_smoke, require_submit,
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
        // `route_layer` runs after routing, so `MatchedPath` is available for the
        // route label; the fallback is intentionally unmetered.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .with_state(state)
}

/// The API router.
pub fn api_router(state: AppState) -> Router {
    let user = Router::new()
        .route("/v1/users/{userId}/notifications", get(feed))
        .route(
            "/v1/users/{userId}/notifications/preferences",
            get(get_preferences).put(put_preferences),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_user));

    let submit = Router::new()
        .route("/v1/alerts", post(submit_alert))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_submit,
        ));

    let anonymise_route = Router::new()
        .route("/v1/internal/privacy/anonymise", post(anonymise))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_anonymise,
        ));

    let smoke_route = Router::new()
        .route("/v1/internal/smoke", get(smoke))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_smoke));

    let router = public_routes()
        .merge(user)
        .merge(submit)
        .merge(anonymise_route)
        .merge(smoke_route);
    finish(router, state)
}

/// The worker router: C11 runs and probes.
pub fn worker_router(state: AppState) -> Router {
    let runs = Router::new()
        .route("/v1/internal/run/notification-deliver", post(run_deliver))
        .route(
            "/v1/internal/run/notification-retention",
            post(run_retention),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_run));

    let router = public_routes().merge(runs);
    finish(router, state)
}

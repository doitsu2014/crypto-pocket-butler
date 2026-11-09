//! The Audit router. Every path names its identity: service token for the append,
//! check, deletion, and run paths; Supabase JWT plus `admin` or `support` for the
//! operator paths. A path with no rule is not served.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::{
    append_event, approve_support_session, check_support_session, create_export,
    create_privacy_deletion, create_support_session, fallback, health, list_events, metrics, ready,
    report_retention_run, revoke_support_session,
};
use crate::middleware::{
    correlation_id, limit_appends, limit_exports, limit_queries, record_metrics, require_admin,
    require_service, require_support, security_headers,
};
use crate::state::AppState;

/// Builds the router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let appends = Router::new()
        .route("/v1/events", post(append_event))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ))
        .route_layer(middleware::from_fn_with_state(state.clone(), limit_appends));

    let queries = Router::new()
        .route("/v1/events", get(list_events))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
        .route_layer(middleware::from_fn_with_state(state.clone(), limit_queries));

    let exports = Router::new()
        .route("/v1/exports", post(create_export))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
        .route_layer(middleware::from_fn_with_state(state.clone(), limit_exports));

    let support_start = Router::new()
        .route("/v1/support-sessions", post(create_support_session))
        .route(
            "/v1/support-sessions/{id}/approve",
            post(approve_support_session),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_support,
        ));

    let support_revoke = Router::new()
        .route(
            "/v1/support-sessions/{id}/revoke",
            post(revoke_support_session),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));

    let internal = Router::new()
        .route(
            "/v1/support-sessions/{id}/check",
            get(check_support_session),
        )
        .route("/v1/privacy/deletions", post(create_privacy_deletion))
        .route("/v1/runs", post(report_retention_run))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ));

    Router::new()
        .merge(public)
        .merge(appends)
        .merge(queries)
        .merge(exports)
        .merge(support_start)
        .merge(support_revoke)
        .merge(internal)
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

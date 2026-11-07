//! The Identity router. Every authenticated path names its identity: the
//! forwarded Supabase JWT for the user surface, a service token for the internal
//! Job Orchestration and support-check paths. A path with no rule is not served.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::handlers::{
    append_consent, approve_support_grant, check_support_grant, fallback, get_consent,
    get_deletion, get_export, health, list_sessions, metrics, ready, request_deletion,
    request_export, request_support_grant, resolve_principal, revoke_session, revoke_support_grant,
    run_privacy_sweep, signup_gate, update_preferences,
};
use crate::middleware::{
    correlation_id, record_metrics, require_jwt, require_service, security_headers,
};
use crate::state::AppState;

/// Builds the router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics))
        .route("/v1/signup-gate", post(signup_gate));

    let authenticated = Router::new()
        .route("/v1/principals/{authUserId}", get(resolve_principal))
        .route("/v1/consent", post(append_consent))
        .route("/v1/consent/{userId}", get(get_consent))
        .route("/v1/users/{userId}/preferences", patch(update_preferences))
        .route("/v1/users/{userId}/sessions", get(list_sessions))
        .route(
            "/v1/users/{userId}/sessions/{sessionId}",
            delete(revoke_session),
        )
        .route("/v1/exports", post(request_export))
        .route("/v1/exports/{exportId}", get(get_export))
        .route("/v1/privacy/deletions", post(request_deletion))
        .route("/v1/privacy/deletions/{deletionId}", get(get_deletion))
        .route("/v1/support-grants", post(request_support_grant))
        .route(
            "/v1/support-grants/{grantId}/approve",
            post(approve_support_grant),
        )
        .route("/v1/support-grants/{grantId}", delete(revoke_support_grant))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    let internal = Router::new()
        .route(
            "/v1/internal/run/audit-privacy-sweep",
            post(run_privacy_sweep),
        )
        .route(
            "/v1/internal/support-grants/{grantId}/check",
            get(check_support_grant),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ));

    Router::new()
        .merge(public)
        .merge(authenticated)
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

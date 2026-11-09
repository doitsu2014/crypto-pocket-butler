//! The two routers. The API serves C7 reads and writes, privacy, and smoke on
//! 8097. The worker serves the C11 run paths and probes on 8098. Every path
//! names its identity; a path with no rule is not served.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::{
    fallback, get_pnl, health, list_transactions, metrics, privacy_anonymise, privacy_export,
    ready, record_trade, run_ingest, run_reconcile, smoke, submit_correction,
};
use crate::middleware::{
    correlation_id, record_metrics, require_run, require_service, require_smoke, require_trades,
    require_user, require_user_or_pnl_service, security_headers,
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
        .route("/v1/users/{userId}/transactions", get(list_transactions))
        .route(
            "/v1/transactions/{transactionId}/corrections",
            post(submit_correction),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_user));

    // C1 (SPA JWT, owner-only) and C7 (Reporting service token, `pnl.read`).
    let pnl = Router::new()
        .route("/v1/users/{userId}/pnl", get(get_pnl))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_user_or_pnl_service,
        ));

    let trades = Router::new()
        .route("/v1/users/{userId}/trades", post(record_trade))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_trades,
        ));

    let privacy = Router::new()
        .route("/v1/internal/privacy/export", get(privacy_export))
        .route("/v1/internal/privacy/anonymise", post(privacy_anonymise))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ));

    let smoke = Router::new()
        .route("/v1/internal/smoke", get(smoke))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_smoke));

    let router = public_routes()
        .merge(user)
        .merge(pnl)
        .merge(trades)
        .merge(privacy)
        .merge(smoke);
    finish(router, state)
}

/// The worker router: C11 runs and probes.
pub fn worker_router(state: AppState) -> Router {
    let runs = Router::new()
        .route("/v1/internal/run/ledger-reconcile", post(run_reconcile))
        .route("/v1/internal/run/ledger-ingest", post(run_ingest))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_run));

    let router = public_routes().merge(runs);
    finish(router, state)
}

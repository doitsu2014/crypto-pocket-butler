//! The Account router. Every authenticated path names its identity: the forwarded
//! Supabase JWT for the user surface, a service token for the internal C5 and C11
//! paths. A path with no rule is not served. The internal
//! `/v1/internal/support-metadata/{userId}` path serves a support grant (BR3.9).

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::{
    add_exchange, add_wallet, add_walletconnect, fallback, get_holdings, get_import,
    get_transactions, health, list_accounts, metrics, privacy_anonymise, privacy_export, ready,
    resync, run_account_sync, support_metadata, upload_import,
};
use crate::middleware::{
    correlation_id, record_metrics, require_jwt, require_service, security_headers,
};
use crate::state::AppState;

/// The CSV upload cap plus room for a small JSON envelope.
pub const MAX_REQUEST_BYTES: usize = 11 * 1024 * 1024;

/// Builds the router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let authenticated = Router::new()
        .route("/v1/accounts/exchange", post(add_exchange))
        .route("/v1/accounts/wallet", post(add_wallet))
        .route("/v1/accounts/walletconnect", post(add_walletconnect))
        .route("/v1/accounts/{accountId}/resync", post(resync))
        .route("/v1/accounts/{accountId}/imports", post(upload_import))
        .route("/v1/imports/{batchId}", get(get_import))
        .route("/v1/users/{userId}/accounts", get(list_accounts))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    let internal = Router::new()
        .route("/v1/users/{userId}/holdings", get(get_holdings))
        .route(
            "/v1/accounts/{accountId}/transactions",
            get(get_transactions),
        )
        .route("/v1/internal/privacy/export", get(privacy_export))
        .route("/v1/internal/privacy/anonymise", post(privacy_anonymise))
        .route(
            "/v1/internal/support-metadata/{userId}",
            get(support_metadata),
        )
        .route("/v1/internal/run/{jobName}", post(run_account_sync))
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

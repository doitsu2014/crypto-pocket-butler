//! The Portfolio routers. Every authenticated path names its identity: the
//! forwarded Supabase JWT for the SPA surface, a service token for the internal
//! C8 and C11 paths. A path with no rule is not served.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;

use crate::handlers::{
    assign_account, cancel_intent, create_portfolio, fallback, get_portfolio,
    get_portfolio_valuation, get_snapshot, get_snapshots, health, list_portfolios,
    list_user_portfolios, metrics, order_intents, privacy_anonymise, privacy_export, put_targets,
    ready, record_trade, smoke,
};
use crate::middleware::{
    correlation_id, record_metrics, require_jwt, require_jwt_or_service, require_service,
    security_headers,
};
use crate::state::AppState;

/// The JSON request cap.
pub const MAX_REQUEST_BYTES: usize = 256 * 1024;

/// Builds the API router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let authenticated = Router::new()
        .route(
            "/v1/portfolios",
            get(list_portfolios).post(create_portfolio),
        )
        .route("/v1/portfolios/{portfolioId}", get(get_portfolio))
        .route(
            "/v1/portfolios/{portfolioId}/accounts/{accountId}",
            put(assign_account),
        )
        .route("/v1/portfolios/{portfolioId}/targets", put(put_targets))
        .route(
            "/v1/portfolios/{portfolioId}/order-intents",
            post(order_intents),
        )
        .route(
            "/v1/portfolios/{portfolioId}/order-intents/{intentId}/cancel",
            post(cancel_intent),
        )
        .route("/v1/portfolios/{portfolioId}/trades", post(record_trade))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    // C1 (SPA JWT) and C8 (Reporting service token) share this path.
    let snapshots = Router::new()
        .route("/v1/portfolios/{portfolioId}/snapshots", get(get_snapshots))
        .route(
            "/v1/portfolios/{portfolioId}/snapshots/{snapshotId}",
            get(get_snapshot),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_jwt_or_service,
        ));

    let internal = Router::new()
        .route("/v1/users/{userId}/portfolios", get(list_user_portfolios))
        .route(
            "/v1/portfolios/{portfolioId}/valuation",
            get(get_portfolio_valuation),
        )
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
        .merge(snapshots)
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

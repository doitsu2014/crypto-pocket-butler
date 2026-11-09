//! Route handlers. Every refusal is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use market_data_application::{AppError, FieldError, ServiceCaller};
use market_data_domain::RefreshJob;

use crate::dto::{FxQuery, FxRateDto, PriceDto, RunResultDto};
use crate::middleware::{CorrelationId, ServicePrincipal};
use crate::problem::{from_app_error, validation, ProblemDetails};
use crate::state::AppState;

const RUN_TRIGGER_HEADER: &str = "x-run-trigger";

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(serde_json::json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe: the database.
pub async fn ready(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(serde_json::json!({ "status": "ready" })).into_response(),
        Err(detail) => ProblemDetails::new(
            503,
            "not-ready",
            "Service not ready",
            Some(detail),
            &correlation.0,
        )
        .into_response(),
    }
}

/// Prometheus text metrics.
pub async fn metrics(State(state): State<AppState>) -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(state.service_name),
    )
        .into_response()
}

/// C6: `getPrices`.
pub async fn get_prices(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    query: Query<Vec<(String, String)>>,
) -> Response {
    let asset_ids = match parse_asset_ids(&query) {
        Ok(asset_ids) => asset_ids,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = ServiceCaller::new(principal.name, principal.operations, &correlation.0);
    match state.quotes.get_prices(&caller, &asset_ids).await {
        Ok(quotes) => Json(quotes.iter().map(PriceDto::from).collect::<Vec<_>>()).into_response(),
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// C6: `getFxRate`.
pub async fn get_fx(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Query(query): Query<FxQuery>,
) -> Response {
    let Some(base) = query.base else {
        return validation(&correlation.0, vec![FieldError::new("base", "is required")])
            .into_response();
    };
    let Some(quote) = query.quote else {
        return validation(
            &correlation.0,
            vec![FieldError::new("quote", "is required")],
        )
        .into_response();
    };
    let caller = ServiceCaller::new(principal.name, principal.operations, &correlation.0);
    match state.fx.get_rate(&caller, &base, &quote).await {
        Ok(rate) => Json(FxRateDto::from(&rate)).into_response(),
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// C11: the refresh run endpoint.
pub async fn run_refresh(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Path(job_name): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(job) = RefreshJob::parse_job_name(&job_name) else {
        return ProblemDetails::not_found(&correlation.0, "the job is not provided by Market Data")
            .into_response();
    };
    let trigger = match headers
        .get(RUN_TRIGGER_HEADER)
        .map(|value| value.to_str().unwrap_or_default())
    {
        Some(raw) => match market_data_domain::Trigger::parse(raw) {
            Ok(trigger) => trigger,
            Err(error) => {
                return from_app_error(&AppError::from_domain(error), &correlation.0)
                    .into_response();
            }
        },
        None => market_data_domain::Trigger::Scheduled,
    };
    let caller = ServiceCaller::new(principal.name, principal.operations, &correlation.0);
    match state
        .refresh
        .run(&caller, job, trigger, &correlation.0)
        .await
    {
        Ok(outcome) => {
            let status = match outcome {
                market_data_application::RunOutcome::Completed(_) => StatusCode::OK,
                market_data_application::RunOutcome::AlreadyRunning(_) => StatusCode::ACCEPTED,
            };
            (status, Json(RunResultDto::from_outcome(&outcome))).into_response()
        }
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// The fallback for an unknown route.
pub async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}

fn parse_asset_ids(query: &[(String, String)]) -> Result<Vec<String>, FieldError> {
    let mut asset_ids = Vec::new();
    for (key, value) in query {
        if key != "assetIds" {
            continue;
        }
        for part in value.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                asset_ids.push(trimmed.to_string());
            }
        }
    }
    if asset_ids.is_empty() {
        return Err(FieldError::new(
            "assetIds",
            "must be present and must not be empty",
        ));
    }
    Ok(asset_ids)
}

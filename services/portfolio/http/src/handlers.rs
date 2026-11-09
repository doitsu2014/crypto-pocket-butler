//! Route handlers. Every refusal is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use portfolio_application::common::{AppError, Caller, FieldError, ServiceCaller};
use portfolio_application::ports::RegisteredService;
use portfolio_domain::{AccountId, OrderIntentId, PortfolioId, UserId};
use serde::Deserialize;
use serde_json::json;

use crate::dto::{
    AnonymiseRequest, AnonymiseResponse, CreatePortfolioRequest, OrderIntentDto,
    OrderIntentPreviewDto, OrderIntentRequestDto, PortfolioDto, PortfolioValuationDto,
    PrivacyExportDto, RecordedTradeDto, SnapshotDetailDto, SnapshotDto, TargetPreviewDto,
    TargetSetRequest, TradeRequestDto,
};
use crate::middleware::{CorrelationId, ServicePrincipal, ValidatedClaims};
use crate::problem::{from_app_error, validation, ProblemDetails};
use crate::state::AppState;

fn app_error(error: AppError, correlation: &str) -> Response {
    from_app_error(&error, correlation).into_response()
}

fn field(field: &'static str, reason: impl Into<String>) -> FieldError {
    FieldError::new(field, reason)
}

fn parse_instant(
    raw: Option<&str>,
    field_name: &'static str,
    correlation: &str,
) -> Result<Option<DateTime<Utc>>, Box<Response>> {
    match raw {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(value) => Ok(Some(value.with_timezone(&Utc))),
            Err(_) => Err(Box::new(
                validation(
                    correlation,
                    vec![field(field_name, "is not an RFC 3339 timestamp")],
                )
                .into_response(),
            )),
        },
        None => Ok(None),
    }
}

fn parse_portfolio_id(raw: &str) -> Result<PortfolioId, FieldError> {
    PortfolioId::parse(raw).map_err(|_| field("portfolioId", "is not a UUID"))
}

fn parse_account_id(raw: &str) -> Result<AccountId, FieldError> {
    AccountId::parse(raw).map_err(|_| field("accountId", "is not a UUID"))
}

fn parse_intent_id(raw: &str) -> Result<OrderIntentId, FieldError> {
    OrderIntentId::parse(raw).map_err(|_| field("intentId", "is not a UUID"))
}

fn parse_user_id(raw: &str) -> Result<UserId, FieldError> {
    UserId::parse(raw).map_err(|_| field("userId", "is not a UUID"))
}

fn caller_of(claims: &portfolio_application::common::VerifiedClaims, correlation: &str) -> Caller {
    Caller::from_claims(claims, correlation)
}

fn service_caller(principal: &RegisteredService, correlation: &str) -> ServiceCaller {
    ServiceCaller::new(principal.name.clone(), correlation)
}

fn body_error(correlation: &str, rejection: JsonRejection) -> Response {
    validation(correlation, vec![field("body", rejection.body_text())]).into_response()
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe: Postgres and its migrations (C5/C6 outages do not fail it).
pub async fn ready(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(json!({ "status": "ready" })).into_response(),
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

/// WF1: list the caller's portfolios, creating the default one on demand.
pub async fn list_portfolios(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = caller_of(&claims, &correlation.0);
    match state.portfolios.list_or_create(&caller).await {
        Ok(portfolios) => Json(
            portfolios
                .iter()
                .map(PortfolioDto::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF1: create a portfolio.
pub async fn create_portfolio(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<CreatePortfolioRequest>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(request)) => request,
        Err(rejection) => return body_error(&correlation.0, rejection),
    };
    let caller = caller_of(&claims, &correlation.0);
    match state
        .portfolios
        .create(
            &caller,
            request.name.as_deref(),
            request.base_currency.as_deref(),
        )
        .await
    {
        Ok(portfolio) => {
            (StatusCode::CREATED, Json(PortfolioDto::from(&portfolio))).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF2: one portfolio with its valuation, breakdown, and drift.
pub async fn get_portfolio(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(portfolio_id): Path<String>,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    match state.valuation.read(&caller, portfolio_id).await {
        Ok(read) => {
            state.metrics.record_valuation(
                read.valuation.status().as_str(),
                read.valuation.unpriced_count(),
            );
            if read.digest_mismatch {
                state.metrics.record_digest_mismatch();
            }
            Json(PortfolioValuationDto::from(&read)).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF1: assign or move an account.
pub async fn assign_account(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path((portfolio_id, account_id)): Path<(String, String)>,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let account_id = match parse_account_id(&account_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    match state
        .portfolios
        .assign_account(&caller, portfolio_id, account_id)
        .await
    {
        Ok(membership) => Json(json!({
            "portfolioId": membership.portfolio_id().to_string(),
            "accountId": membership.account_id().to_string(),
            "addedAt": membership.added_at(),
        }))
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// Query string for a preview.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewQuery {
    /// Whether the request is a preview.
    #[serde(default)]
    pub preview: Option<bool>,
}

/// WF3: preview or save targets and guardrails.
pub async fn put_targets(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(portfolio_id): Path<String>,
    Query(query): Query<PreviewQuery>,
    payload: Result<Json<TargetSetRequest>, JsonRejection>,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let request = match payload {
        Ok(Json(request)) => request,
        Err(rejection) => return body_error(&correlation.0, rejection),
    };
    let inputs = request.into_inputs();
    let caller = caller_of(&claims, &correlation.0);
    if query.preview.unwrap_or(false) {
        return match state.targets.preview(portfolio_id, &inputs) {
            Ok(preview) => Json(TargetPreviewDto {
                sum: preview.sum,
                difference: preview.difference,
                saved: false,
            })
            .into_response(),
            Err(error) => app_error(error, &correlation.0),
        };
    }
    match state.targets.save(&caller, portfolio_id, &inputs).await {
        Ok(saved) => Json(TargetPreviewDto {
            sum: saved.preview.sum,
            difference: saved.preview.difference,
            saved: true,
        })
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF5: preview or record an order intent.
pub async fn order_intents(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(portfolio_id): Path<String>,
    Query(query): Query<PreviewQuery>,
    payload: Result<Json<OrderIntentRequestDto>, JsonRejection>,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let request = match payload {
        Ok(Json(request)) => request,
        Err(rejection) => return body_error(&correlation.0, rejection),
    };
    let request = match request.into_request() {
        Ok(request) => request,
        Err((field_name, reason)) => {
            return validation(&correlation.0, vec![field(field_name, reason)]).into_response()
        }
    };
    let caller = caller_of(&claims, &correlation.0);
    if query.preview.unwrap_or(false) {
        return match state.intents.preview(&caller, portfolio_id, &request).await {
            Ok(projection) => Json(OrderIntentPreviewDto::from(&projection)).into_response(),
            Err(error) => app_error(error, &correlation.0),
        };
    }
    match state.intents.create(&caller, portfolio_id, &request).await {
        Ok(intent) => (StatusCode::CREATED, Json(OrderIntentDto::from(&intent))).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF5: cancel a pending intent.
pub async fn cancel_intent(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path((portfolio_id, intent_id)): Path<(String, String)>,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let intent_id = match parse_intent_id(&intent_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    match state.intents.cancel(&caller, portfolio_id, intent_id).await {
        Ok(intent) => Json(OrderIntentDto::from(&intent)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF6: record an executed trade.
pub async fn record_trade(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(portfolio_id): Path<String>,
    payload: Result<Json<TradeRequestDto>, JsonRejection>,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let request = match payload {
        Ok(Json(request)) => request,
        Err(rejection) => return body_error(&correlation.0, rejection),
    };
    let request = match request.into_request() {
        Ok(request) => request,
        Err((field_name, reason)) => {
            return validation(&correlation.0, vec![field(field_name, reason)]).into_response()
        }
    };
    let caller = caller_of(&claims, &correlation.0);
    let portfolio = match state.valuation.owned(&caller, portfolio_id).await {
        Ok(portfolio) => portfolio,
        Err(error) => return app_error(error, &correlation.0),
    };
    match state.trades.record(&caller, portfolio_id, &request).await {
        Ok(trade) => {
            // NFR1.5 / BR6.1: the post-trade recompute also evaluates the
            // guardrails. A failure here never fails the recorded trade.
            match state.valuation.fresh(&portfolio).await {
                Ok(fresh) => match state
                    .guardrails
                    .evaluate(&portfolio, &fresh.valuation)
                    .await
                {
                    Ok(report) => {
                        state
                            .metrics
                            .record_guardrail_alert_failure(report.alerts_failed);
                    }
                    Err(error) => tracing::warn!(%error, "post-trade guardrail evaluation failed"),
                },
                Err(error) => tracing::warn!(%error, "post-trade valuation failed"),
            }
            (StatusCode::CREATED, Json(RecordedTradeDto::from(&trade))).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// Query string for the snapshot history.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Only snapshots at or after this RFC 3339 instant.
    #[serde(default)]
    pub from: Option<String>,
    /// The page size; 100 by default, capped at 500.
    #[serde(default)]
    pub limit: Option<i64>,
    /// The `as_of` cursor returned in `x-next-cursor` by the previous page.
    #[serde(default)]
    pub cursor: Option<String>,
}

/// WF4 / C8: snapshot history. The SPA presents a JWT; Reporting presents a
/// service token on the same path. The response is bounded and carries
/// `x-next-cursor` and `x-has-more`; reproduction is on demand per snapshot.
pub async fn get_snapshots(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Path(portfolio_id): Path<String>,
    Query(query): Query<HistoryQuery>,
    request: Request,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let from = match parse_instant(query.from.as_deref(), "from", &correlation.0) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let cursor = match parse_instant(query.cursor.as_deref(), "cursor", &correlation.0) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let limit = query
        .limit
        .unwrap_or(portfolio_application::valuation::DEFAULT_HISTORY_PAGE)
        .clamp(1, portfolio_application::valuation::MAX_HISTORY_PAGE);
    let page = if let Some(ServicePrincipal(principal)) =
        request.extensions().get::<ServicePrincipal>()
    {
        let caller = service_caller(principal, &correlation.0);
        state
            .valuation
            .history_for_service(&caller, portfolio_id, from, limit, cursor)
            .await
    } else if let Some(ValidatedClaims(claims)) = request.extensions().get::<ValidatedClaims>() {
        let caller = caller_of(claims, &correlation.0);
        match state.valuation.owned(&caller, portfolio_id).await {
            Ok(portfolio) => {
                state
                    .valuation
                    .history(portfolio.id(), from, limit, cursor)
                    .await
            }
            Err(error) => return app_error(error, &correlation.0),
        }
    } else {
        return ProblemDetails::new(
            401,
            "unauthorized",
            "Unauthorized",
            Some("a bearer token or a service token is required".to_string()),
            &correlation.0,
        )
        .into_response();
    };
    match page {
        Ok(page) => {
            let mut headers = HeaderMap::new();
            if let Some(cursor) = page.next_cursor {
                if let Ok(value) = HeaderValue::from_str(&cursor.to_rfc3339()) {
                    headers.insert("x-next-cursor", value);
                }
            }
            headers.insert(
                "x-has-more",
                HeaderValue::from_static(if page.has_more { "true" } else { "false" }),
            );
            (
                headers,
                Json(
                    page.snapshots
                        .iter()
                        .map(SnapshotDto::from)
                        .collect::<Vec<_>>(),
                ),
            )
                .into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF4 / C8: reproduce one snapshot on demand.
pub async fn get_snapshot(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Path((portfolio_id, snapshot_id)): Path<(String, String)>,
    request: Request,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let snapshot_id = match portfolio_domain::ValuationSnapshotId::parse(&snapshot_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(&correlation.0, vec![field("snapshotId", "is not a UUID")])
                .into_response()
        }
    };
    let owned = if let Some(ServicePrincipal(principal)) =
        request.extensions().get::<ServicePrincipal>()
    {
        let caller = service_caller(principal, &correlation.0);
        match caller.require_operation(portfolio_domain::Operation::SnapshotsRead) {
            Ok(()) => true,
            Err(error) => return app_error(error, &correlation.0),
        }
    } else if let Some(ValidatedClaims(claims)) = request.extensions().get::<ValidatedClaims>() {
        let caller = caller_of(claims, &correlation.0);
        match state.valuation.owned(&caller, portfolio_id).await {
            Ok(_) => true,
            Err(error) => return app_error(error, &correlation.0),
        }
    } else {
        return ProblemDetails::new(
            401,
            "unauthorized",
            "Unauthorized",
            Some("a bearer token or a service token is required".to_string()),
            &correlation.0,
        )
        .into_response();
    };
    if !owned {
        return ProblemDetails::not_found(&correlation.0, "snapshot was not found").into_response();
    }
    match state
        .valuation
        .snapshot_detail(portfolio_id, snapshot_id)
        .await
    {
        Ok(Some(detail)) => {
            if !detail.verified {
                state.metrics.record_digest_mismatch();
            }
            Json(SnapshotDetailDto::from(&detail)).into_response()
        }
        Ok(None) => {
            ProblemDetails::not_found(&correlation.0, "snapshot was not found").into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C8: the user's portfolios for the Reporting service.
pub async fn list_user_portfolios(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Path(user_id): Path<String>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = service_caller(&principal, &correlation.0);
    if let Err(error) = caller.require_operation(portfolio_domain::Operation::PortfoliosRead) {
        return app_error(error, &correlation.0);
    }
    match state.portfolios.list_for_user(user_id).await {
        Ok(portfolios) => {
            let body: Vec<PortfolioDto> = portfolios.iter().map(PortfolioDto::from).collect();
            Json(body).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C8: one portfolio's valuation with its lines for the Reporting service.
pub async fn get_portfolio_valuation(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Path(portfolio_id): Path<String>,
) -> Response {
    let portfolio_id = match parse_portfolio_id(&portfolio_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = service_caller(&principal, &correlation.0);
    if let Err(error) = caller.require_operation(portfolio_domain::Operation::ValuationRead) {
        return app_error(error, &correlation.0);
    }
    match state.valuation.read_internal(portfolio_id).await {
        Ok(read) => {
            state.metrics.record_valuation(
                read.valuation.status().as_str(),
                read.valuation.unpriced_count(),
            );
            if read.digest_mismatch {
                state.metrics.record_digest_mismatch();
            }
            Json(PortfolioValuationDto::from(&read)).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// Query string for the privacy export.
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// The user to export.
    #[serde(rename = "userId")]
    pub user_id: String,
}

/// C8: the portfolio data Identity assembles for a user.
pub async fn privacy_export(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Query(query): Query<ExportQuery>,
) -> Response {
    let user_id = match parse_user_id(&query.user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = service_caller(&principal, &correlation.0);
    match state.privacy.export(&caller, user_id).await {
        Ok(export) => Json(PrivacyExportDto::from(export)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C8: anonymise a user's portfolio data for Identity.
pub async fn privacy_anonymise(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    payload: Result<Json<AnonymiseRequest>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(request)) => request,
        Err(rejection) => return body_error(&correlation.0, rejection),
    };
    let user_id = match parse_user_id(&request.user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = service_caller(&principal, &correlation.0);
    match state
        .privacy
        .anonymise(&caller, user_id, &request.idempotency_key)
        .await
    {
        Ok(result) => Json(AnonymiseResponse {
            user_id: result.user_id.to_string(),
            retained_categories: result.retained_categories,
            already_applied: result.already_applied,
        })
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// The release smoke endpoint. A registered service token (the smoke-reader)
/// confirms the API is the Portfolio service and answers.
pub async fn smoke(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
) -> Response {
    Json(json!({
        "service": state.service_name,
        "caller": principal.name,
        "status": "ok",
        "correlationId": correlation.0,
    }))
    .into_response()
}

/// The fallback for an unknown route.
pub async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}

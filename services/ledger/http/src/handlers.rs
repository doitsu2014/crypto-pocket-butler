//! Route handlers. Every failure is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use std::time::Instant;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use ledger_application::{AppError, FieldError, RepositoryError};
use ledger_domain::{Kind, TransactionId};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::dto::{
    AnonymiseRequest, AnonymiseResponse, CorrectionDto, CorrectionRequest, IngestResponse,
    IngestRunRequest, PnlDto, PrivacyExportDto, ReconcileResponse, TradeRequest, TradeResponse,
    TransactionPageResponse,
};
use crate::middleware::{CorrelationId, ServicePrincipal};
use crate::problem::{FieldProblem, ProblemDetails};
use crate::state::AppState;

fn problem(
    status: u16,
    problem_type: &str,
    title: &str,
    detail: impl Into<String>,
    correlation: &str,
) -> Response {
    ProblemDetails::new(
        status,
        problem_type,
        title,
        Some(detail.into()),
        correlation,
    )
    .into_response()
}

fn fields(errors: Vec<FieldError>) -> Vec<FieldProblem> {
    errors
        .into_iter()
        .map(|error| FieldProblem {
            field: error.field,
            reason: error.reason,
        })
        .collect()
}

fn validation(correlation: &str, errors: Vec<FieldError>) -> Response {
    ProblemDetails::new(
        422,
        "validation",
        "Invalid request",
        Some("one or more fields failed validation".to_string()),
        correlation,
    )
    .with_fields(fields(errors))
    .into_response()
}

fn app_error(error: AppError, correlation: &str) -> Response {
    match error {
        AppError::Rejected(errors) => validation(correlation, errors),
        AppError::NotFound => problem(
            404,
            "not-found",
            "Not found",
            "no such transaction",
            correlation,
        ),
        AppError::Forbidden => problem(
            403,
            "forbidden",
            "Forbidden",
            "the caller may not perform this operation",
            correlation,
        ),
        AppError::Conflict(_) => problem(
            409,
            "conflict",
            "Conflict",
            "the request conflicts with a record already in the ledger",
            correlation,
        ),
        AppError::AssetUnavailable(_) => problem(
            503,
            "asset-unavailable",
            "Asset resolution unavailable",
            "the asset registry could not resolve the asset",
            correlation,
        ),
        AppError::Repository(RepositoryError::Forbidden(_)) => problem(
            403,
            "grant-refused",
            "Refused by the database",
            "the database refused the operation",
            correlation,
        ),
        AppError::Repository(_) => problem(
            503,
            "unavailable",
            "Service unavailable",
            "the ledger store is unavailable",
            correlation,
        ),
        AppError::External(_) => problem(
            503,
            "unavailable",
            "Service unavailable",
            "a dependency is unavailable",
            correlation,
        ),
    }
}

fn body_error(correlation: &str) -> Response {
    validation(
        correlation,
        vec![FieldError::new(
            "body",
            "must be a JSON object matching the contract",
        )],
    )
}

fn parse_user(user_id: &str, correlation: &str) -> Result<String, Box<Response>> {
    Uuid::parse_str(user_id)
        .map(|value| value.to_string())
        .map_err(|_| {
            Box::new(validation(
                correlation,
                vec![FieldError::new("userId", "must be a UUID")],
            ))
        })
}

fn parse_decimal(field: &str, raw: &str, correlation: &str) -> Result<Decimal, Box<Response>> {
    raw.trim().parse::<Decimal>().map_err(|_| {
        Box::new(validation(
            correlation,
            vec![FieldError::new(field, "must be a decimal string")],
        ))
    })
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe: Postgres only.
pub async fn ready(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(json!({ "status": "ready" })).into_response(),
        Err(detail) => problem(
            503,
            "not-ready",
            "Service not ready",
            detail,
            &correlation.0,
        ),
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

/// Query parameters for the transaction list.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListParams {
    /// The keyset cursor from the previous page.
    #[serde(default)]
    pub cursor: Option<String>,
    /// The page size.
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Query parameters for the P&L read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlParams {
    /// The optional portfolio scope.
    #[serde(default)]
    pub portfolio_id: Option<String>,
}

/// Query parameters for the privacy export.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    /// The user to export.
    pub user_id: String,
}

/// C7: list a user's transactions, newest first.
pub async fn list_transactions(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<ledger_application::Principal>,
    Path(user_id): Path<String>,
    Query(params): Query<ListParams>,
) -> Response {
    let user_id = match parse_user(&user_id, &correlation.0) {
        Ok(user_id) => user_id,
        Err(response) => return *response,
    };
    if principal.subject != user_id {
        return problem(
            403,
            "forbidden",
            "Forbidden",
            "a user may read only their own ledger",
            &correlation.0,
        );
    }
    match state
        .reads
        .transactions(&user_id, params.cursor.as_deref(), params.limit)
        .await
    {
        Ok(page) => Json(TransactionPageResponse::from_page(page)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C7: read a user's P&L with its coverage. A forwarded JWT reads only the
/// caller's own ledger; Reporting's service token (`pnl.read`) may read any
/// user it is composing a brief for.
pub async fn get_pnl(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    principal: Option<axum::extract::Extension<ledger_application::Principal>>,
    _service: Option<axum::extract::Extension<ServicePrincipal>>,
    Path(user_id): Path<String>,
    Query(params): Query<PnlParams>,
) -> Response {
    let user_id = match parse_user(&user_id, &correlation.0) {
        Ok(user_id) => user_id,
        Err(response) => return *response,
    };
    let principal = principal.map(|axum::extract::Extension(principal)| principal);
    let has_service = _service.is_some();
    match (has_service, principal) {
        // Reporting's token already passed the `pnl.read` check in middleware.
        (true, _) => {}
        (false, Some(principal)) if principal.subject == user_id => {}
        (false, Some(_)) => {
            return problem(
                403,
                "forbidden",
                "Forbidden",
                "a user may read only their own ledger",
                &correlation.0,
            );
        }
        (false, None) => {
            return problem(
                401,
                "unauthorized",
                "Unauthorized",
                "a bearer token is required",
                &correlation.0,
            );
        }
    }
    let portfolio_id = match params.portfolio_id.as_deref() {
        None => None,
        Some(raw) => match Uuid::parse_str(raw) {
            Ok(id) => Some(id),
            Err(_) => {
                return validation(
                    &correlation.0,
                    vec![FieldError::new("portfolioId", "must be a UUID")],
                )
            }
        },
    };
    match state.pnl.report(&user_id, portfolio_id).await {
        Ok(report) => {
            state.metrics.record_coverage(
                report
                    .coverage
                    .to_string()
                    .parse::<f64>()
                    .unwrap_or_default(),
            );
            Json(PnlDto::from_report(&report)).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF5: record a correction.
pub async fn submit_correction(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<ledger_application::Principal>,
    Path(transaction_id): Path<String>,
    payload: Result<Json<CorrectionRequest>, JsonRejection>,
) -> Response {
    let transaction_id = match TransactionId::parse(&transaction_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("transactionId", "must be a UUID")],
            )
        }
    };
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return body_error(&correlation.0),
    };
    match state
        .corrections
        .correct(
            &principal.subject,
            transaction_id,
            &request.field,
            &request.new_value,
            &request.reason,
        )
        .await
    {
        Ok(result) => {
            state.metrics.record_correction(&request.field);
            if result.audit_failed {
                state.metrics.record_audit_failure();
            }
            (
                StatusCode::CREATED,
                Json(CorrectionDto::from_result(&result)),
            )
                .into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF6: record an executed trade submitted by Portfolio.
pub async fn record_trade(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    Path(user_id): Path<String>,
    payload: Result<Json<TradeRequest>, JsonRejection>,
) -> Response {
    let user_id = match parse_user(&user_id, &correlation.0) {
        Ok(user_id) => user_id,
        Err(response) => return *response,
    };
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return body_error(&correlation.0),
    };
    let kind = match Kind::parse(&request.kind) {
        Ok(kind) => kind,
        Err(error) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("kind", error.to_string())],
            )
        }
    };
    let quantity = match parse_decimal("quantity", &request.quantity, &correlation.0) {
        Ok(quantity) => quantity,
        Err(response) => return *response,
    };
    let price = match request.price.as_deref() {
        None => None,
        Some(raw) => match parse_decimal("price", raw, &correlation.0) {
            Ok(price) => Some(price),
            Err(response) => return *response,
        },
    };
    match state
        .trades
        .record(
            &user_id,
            ledger_application::TradeCommand {
                asset_id: request.asset_id,
                kind,
                quantity,
                price,
                currency: request.currency,
                executed_at: request.executed_at,
            },
        )
        .await
    {
        Ok(trade) => {
            state
                .metrics
                .record_transactions(kind.as_str(), "recorded_trade", 1);
            if trade.audit_failed {
                state.metrics.record_audit_failure();
            }
            (StatusCode::CREATED, Json(TradeResponse::from_trade(&trade))).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C7: export the financial rows for Identity.
pub async fn privacy_export(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(service): axum::extract::Extension<ServicePrincipal>,
    Query(params): Query<ExportParams>,
) -> Response {
    let user_id = match parse_user(&params.user_id, &correlation.0) {
        Ok(user_id) => user_id,
        Err(response) => return *response,
    };
    match state.privacy.export(&service.0, &user_id).await {
        Ok(export) => Json(PrivacyExportDto::from_export(&export)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C7: anonymise a user's mutable subject for Identity.
pub async fn privacy_anonymise(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(service): axum::extract::Extension<ServicePrincipal>,
    payload: Result<Json<AnonymiseRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return body_error(&correlation.0),
    };
    match state
        .privacy
        .anonymise(
            &service.0,
            &request.user_id.to_string(),
            &request.idempotency_key,
        )
        .await
    {
        Ok(result) => {
            if result.audit_failed {
                state.metrics.record_audit_failure();
            }
            Json(AnonymiseResponse::from_result(&result)).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C12: release smoke.
pub async fn smoke(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(json!({ "status": "ready" })).into_response(),
        Err(detail) => problem(
            503,
            "not-ready",
            "Service not ready",
            detail,
            &correlation.0,
        ),
    }
}

/// C11: run the reconcile.
pub async fn run_reconcile(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    let started = Instant::now();
    match state.reconcile.run().await {
        Ok(report) => {
            state
                .metrics
                .record_reconcile(report.status.as_str(), started.elapsed().as_secs_f64());
            Json(ReconcileResponse::from_report(&report)).into_response()
        }
        Err(error) => {
            state
                .metrics
                .record_reconcile("error", started.elapsed().as_secs_f64());
            app_error(error, &correlation.0)
        }
    }
}

/// C11 extension: ingest a user's C5 rows on sync or import completion.
pub async fn run_ingest(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    payload: Result<Json<IngestRunRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return body_error(&correlation.0),
    };
    match state.ingest.ingest_user(&request.user_id.to_string()).await {
        Ok(report) => {
            state.metrics.record_duplicates("mixed", report.duplicates);
            Json(IngestResponse::from_report(&report)).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// Unknown path.
pub async fn fallback(request: Request) -> Response {
    let correlation = crate::middleware::correlation_of(&request);
    problem(
        404,
        "not-found",
        "Not found",
        "no route matches this path",
        &correlation,
    )
}

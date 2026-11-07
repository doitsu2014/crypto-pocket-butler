//! Route handlers. Every refusal is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use account_application::common::{AppError, Caller, FieldError, ServiceCaller};
use account_application::ports::{ExchangeCredentials, RegisteredService};
use account_application::sync::RunOutcome;
use account_domain::{AccountId, ImportBatchId, SyncRun, UserId};
use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::dto::{
    AccountResponse, AddExchangeRequest, AddWalletConnectRequest, AddWalletRequest,
    AnonymiseRequest, AnonymiseResponse, HoldingDto, ImportAcceptedResponse, PrivacyExportResponse,
    ResyncResponse, RunRequest, RunResultResponse, TransactionDto,
};
use crate::middleware::{CorrelationId, ServicePrincipal, ValidatedClaims};
use crate::problem::{from_app_error, validation, ProblemDetails};
use crate::state::AppState;

fn app_error(error: AppError, correlation: &str) -> Response {
    from_app_error(&error, correlation).into_response()
}

fn parse_user_id(raw: &str) -> Result<UserId, FieldError> {
    UserId::parse(raw).map_err(|_| FieldError::new("userId", "is not a UUID"))
}

fn parse_account_id(raw: &str) -> Result<AccountId, FieldError> {
    AccountId::parse(raw).map_err(|_| FieldError::new("accountId", "is not a UUID"))
}

fn parse_batch_id(raw: &str) -> Result<ImportBatchId, FieldError> {
    ImportBatchId::parse(raw).map_err(|_| FieldError::new("batchId", "is not a UUID"))
}

fn caller_of(claims: &account_application::common::VerifiedClaims, correlation: &str) -> Caller {
    Caller::from_claims(claims, correlation)
}

fn service_caller(principal: &RegisteredService, correlation: &str) -> ServiceCaller {
    ServiceCaller::new(principal.name.clone(), correlation)
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe: the database, its migrations, and Vault.
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

/// WF1: add an exchange account with a read-only key.
pub async fn add_exchange(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<AddExchangeRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
    };
    let mut key = ExchangeCredentials::new(request.api_key, request.api_secret);
    if let Some(passphrase) = request.passphrase {
        key = key.with_passphrase(passphrase);
    }
    let caller = caller_of(&claims, &correlation.0);
    match state
        .accounts
        .add_exchange(&caller, &request.venue, &request.label, key)
        .await
    {
        Ok(account) => (StatusCode::CREATED, Json(AccountResponse::from(&account))).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF2: add a public wallet address.
pub async fn add_wallet(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<AddWalletRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
    };
    let caller = caller_of(&claims, &correlation.0);
    match state
        .accounts
        .add_wallet(&caller, &request.chain, &request.address, &request.label)
        .await
    {
        Ok(account) => (StatusCode::CREATED, Json(AccountResponse::from(&account))).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF3: attach a WalletConnect read-only connection.
pub async fn add_walletconnect(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<AddWalletConnectRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
    };
    let caller = caller_of(&claims, &correlation.0);
    match state
        .accounts
        .add_walletconnect(
            &caller,
            &request.chain,
            &request.address,
            &request.label,
            &request.session_ref,
        )
        .await
    {
        Ok(account) => (StatusCode::CREATED, Json(AccountResponse::from(&account))).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C1: manual re-sync. Returns 202 with the active run id (BR4.1).
pub async fn resync(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(account_id): Path<String>,
) -> Response {
    let account_id = match parse_account_id(&account_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    match state.accounts.resync(&caller, account_id).await {
        Ok(outcome) => {
            let run: SyncRun = match outcome {
                account_application::accounts::ResyncOutcome::Started(run)
                | account_application::accounts::ResyncOutcome::AlreadyRunning(run) => run,
            };
            (StatusCode::ACCEPTED, Json(ResyncResponse::from_run(&run))).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C1: list the user's accounts with status and last-synced time.
pub async fn list_accounts(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    match state.accounts.list(&caller, user_id).await {
        Ok(accounts) => Json(
            accounts
                .iter()
                .map(AccountResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C1: upload a CSV to a csv account. The bytes are staged in the service schema.
pub async fn upload_import(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let account_id = match parse_account_id(&account_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let file_name = headers
        .get("x-file-name")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(file_name) = file_name else {
        return validation(
            &correlation.0,
            vec![FieldError::new("x-file-name", "is required")],
        )
        .into_response();
    };
    let caller = caller_of(&claims, &correlation.0);
    match state
        .imports
        .upload(&caller, account_id, file_name, &body)
        .await
    {
        Ok(batch) => (
            StatusCode::ACCEPTED,
            Json(ImportAcceptedResponse {
                batch_id: batch.id().to_string(),
                status: batch.status().as_str().to_string(),
            }),
        )
            .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C5: holdings per account.
pub async fn get_holdings(
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
    match state.holdings.for_user(&caller, user_id).await {
        Ok(accounts) => {
            let holdings: Vec<HoldingDto> = accounts.iter().flat_map(HoldingDto::flatten).collect();
            Json(holdings).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// Query string for the transactions read.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsQuery {
    /// Only rows executed at or after this RFC 3339 instant.
    #[serde(default)]
    pub since: Option<String>,
}

/// C5: venue and imported transactions.
pub async fn get_transactions(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Path(account_id): Path<String>,
    Query(query): Query<TransactionsQuery>,
) -> Response {
    let account_id = match parse_account_id(&account_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let since = match query.since.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(value) => Some(value.with_timezone(&Utc)),
            Err(_) => {
                return validation(
                    &correlation.0,
                    vec![FieldError::new("since", "is not an RFC 3339 timestamp")],
                )
                .into_response();
            }
        },
        None => None,
    };
    let caller = service_caller(&principal, &correlation.0);
    match state
        .holdings
        .transactions(&caller, account_id, since)
        .await
    {
        Ok(transactions) => Json(
            transactions
                .iter()
                .map(TransactionDto::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
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

/// C5: the data export Identity assembles for a user.
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
        Ok(export) => Json(PrivacyExportResponse::from(export)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C5: anonymise a user's account data for Identity.
pub async fn privacy_anonymise(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    payload: Result<Json<AnonymiseRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
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

/// C11: the scheduled account-sync run.
pub async fn run_account_sync(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Path(job_name): Path<String>,
    payload: Result<Json<RunRequest>, JsonRejection>,
) -> Response {
    if job_name != "account-sync" {
        return ProblemDetails::not_found(&correlation.0, "the job is not provided by Account")
            .into_response();
    }
    let request = match payload {
        Ok(Json(request)) => request,
        Err(_) => RunRequest::default(),
    };
    let caller = service_caller(&principal, &correlation.0);
    if let Err(error) = caller.require_operation(account_domain::Operation::RunScheduledSync) {
        return app_error(error, &correlation.0);
    }
    let started_at = Utc::now();
    if request.account_ids.is_empty() {
        return match state.sync.process_pending(&correlation.0).await {
            Ok(report) => {
                let status = if report.failed > 0 {
                    "failed"
                } else if report.processed > 0 {
                    "succeeded"
                } else {
                    "skipped"
                };
                let detail = format!(
                    "processed={} failed={} skipped={}",
                    report.processed, report.failed, report.skipped
                );
                (
                    StatusCode::OK,
                    Json(RunResultResponse::new(status, started_at, Some(detail))),
                )
                    .into_response()
            }
            Err(error) => app_error(error, &correlation.0),
        };
    }

    let mut status = "succeeded";
    let mut detail = String::new();
    for raw in &request.account_ids {
        let account_id = match parse_account_id(raw) {
            Ok(id) => id,
            Err(error) => return validation(&correlation.0, vec![error]).into_response(),
        };
        match state.sync.run_account(account_id, &correlation.0).await {
            Ok(RunOutcome::Succeeded(_)) => {}
            Ok(RunOutcome::Skipped(reason)) => {
                status = "skipped";
                detail = reason.to_string();
            }
            Ok(RunOutcome::Failed(_)) => {
                status = "failed";
                detail = "one or more runs failed".to_string();
            }
            Err(AppError::Conflict(message)) => {
                return ProblemDetails::new(
                    409,
                    "conflict",
                    "Conflict",
                    Some(message),
                    &correlation.0,
                )
                .into_response();
            }
            Err(error) => return app_error(error, &correlation.0),
        }
    }
    (
        StatusCode::OK,
        Json(RunResultResponse::new(
            status,
            started_at,
            Some(detail).filter(|value| !value.is_empty()),
        )),
    )
        .into_response()
}

/// Health of a CSV batch readback.
pub async fn get_import(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(batch_id): Path<String>,
) -> Response {
    let batch_id = match parse_batch_id(&batch_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    match state.imports.find(&caller, batch_id).await {
        Ok(batch) => Json(json!({
            "batchId": batch.id().to_string(),
            "status": batch.status().as_str(),
            "source": batch.source().as_str(),
            "fileName": batch.file_name(),
            "imported": batch.imported_count(),
            "skipped": batch.skipped_count(),
            "failed": batch.failed_count(),
        }))
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// The fallback for an unknown route.
pub async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}

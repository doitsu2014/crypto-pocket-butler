//! Route handlers. Every failure is an RFC 7807 problem carrying the correlation id.

use asset_registry_application::admin::{
    AddAssetCommand, AddTokenContractCommand, AdminActor, AdminError, IgnoreObservationCommand,
    ResolveObservationCommand, UpsertPriceSourceCommand,
};
use asset_registry_application::ports::Principal;
use asset_registry_application::resolve_asset::ResolveOutcome;
use asset_registry_application::resolve_price::PriceQuery;
use asset_registry_domain::AssetId;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::dto::{
    AddAssetRequest, AddTokenContractRequest, AssetDto, IgnoreObservationRequest, ObservationDto,
    PriceResponseDto, PriceSourceDto, ResolveAssetQueryDto, ResolveObservationRequest,
    SetEnabledRequest, TokenContractDto, UpsertPriceSourceRequest,
};
use crate::middleware::CorrelationId;
use crate::problem::ProblemDetails;
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

fn validation(correlation: &str, detail: impl Into<String>) -> Response {
    problem(422, "validation", "Invalid request", detail, correlation)
}

fn not_found(correlation: &str, detail: impl Into<String>) -> Response {
    problem(404, "not-found", "Not found", detail, correlation)
}

fn unavailable(correlation: &str, detail: impl Into<String>) -> Response {
    problem(
        503,
        "service-unavailable",
        "Service unavailable",
        detail,
        correlation,
    )
}

fn admin_error(error: AdminError, correlation: &str) -> Response {
    match error {
        AdminError::Forbidden(detail) => {
            problem(403, "forbidden", "Forbidden", detail, correlation)
        }
        AdminError::Validation(detail) => validation(correlation, detail),
        AdminError::NotFound(detail) => not_found(correlation, detail),
        AdminError::Conflict(detail) => problem(409, "conflict", "Conflict", detail, correlation),
        AdminError::Audit(detail) => problem(
            503,
            "audit-unavailable",
            "Change refused: audit unavailable",
            detail,
            correlation,
        ),
        AdminError::Repository(detail) => unavailable(correlation, detail),
    }
}

fn actor(principal: &Principal) -> Option<AdminActor> {
    AdminActor::new(principal.subject.clone(), principal.role).ok()
}

fn missing_actor(correlation: &str) -> Response {
    problem(
        403,
        "forbidden",
        "Forbidden",
        "authenticated principal is invalid",
        correlation,
    )
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe.
pub async fn ready(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
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

/// Resolve an observation to a canonical asset.
pub async fn resolve_asset(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    query: Result<Query<ResolveAssetQueryDto>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(rejection) => return validation(&correlation.0, rejection.body_text()),
    };

    match state.resolve_asset.execute(query.into()).await {
        Ok(ResolveOutcome::Resolved(asset)) => Json(AssetDto::from(&asset)).into_response(),
        Ok(ResolveOutcome::Unresolved(observation)) => problem(
            404,
            "asset-unresolved",
            "Asset unresolved",
            format!(
                "observation '{}' is {} and is excluded from portfolio totals",
                observation.id(),
                observation.status().label()
            ),
            &correlation.0,
        ),
        Err(error) => {
            use asset_registry_application::resolve_asset::ResolveError;
            match error {
                ResolveError::Repository(repository) => {
                    unavailable(&correlation.0, repository.to_string())
                }
                other => validation(&correlation.0, other.to_string()),
            }
        }
    }
}

/// Resolve a price with priority and fallback.
pub async fn resolve_price(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Path(asset_id): Path<String>,
) -> Response {
    let asset_id = match AssetId::new(asset_id) {
        Ok(asset_id) => asset_id,
        Err(error) => return validation(&correlation.0, error.to_string()),
    };
    let asset = match state.assets.find_asset_by_id(&asset_id).await {
        Ok(Some(asset)) => asset,
        Ok(None) => return not_found(&correlation.0, format!("asset '{asset_id}'")),
        Err(error) => return unavailable(&correlation.0, error.to_string()),
    };
    match state.resolve_price.execute(PriceQuery { asset }).await {
        Ok(outcome) => Json(PriceResponseDto::from(outcome)).into_response(),
        Err(error) => unavailable(&correlation.0, error.to_string()),
    }
}

/// Add an asset.
pub async fn create_asset(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(principal): Extension<Principal>,
    payload: Result<Json<AddAssetRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return validation(&correlation.0, rejection.body_text()),
    };
    let Some(actor) = actor(&principal) else {
        return missing_actor(&correlation.0);
    };
    match state
        .admin
        .add_asset(
            &actor,
            AddAssetCommand {
                id: request.id,
                symbol: request.symbol,
                name: request.name,
                decimals: request.decimals,
                native_chain: request.native_chain,
            },
            Some(&correlation.0),
        )
        .await
    {
        Ok(asset) => (StatusCode::CREATED, Json(AssetDto::from(&asset))).into_response(),
        Err(error) => admin_error(error, &correlation.0),
    }
}

/// Add a chain-plus-contract mapping.
pub async fn create_token_contract(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(principal): Extension<Principal>,
    payload: Result<Json<AddTokenContractRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return validation(&correlation.0, rejection.body_text()),
    };
    let Some(actor) = actor(&principal) else {
        return missing_actor(&correlation.0);
    };
    match state
        .admin
        .add_token_contract(
            &actor,
            AddTokenContractCommand {
                id: request.id,
                chain_id: request.chain_id,
                contract_address: request.contract_address,
                asset_id: request.asset_id,
                decimals: request.decimals,
            },
            Some(&correlation.0),
        )
        .await
    {
        Ok(contract) => {
            (StatusCode::CREATED, Json(TokenContractDto::from(&contract))).into_response()
        }
        Err(error) => admin_error(error, &correlation.0),
    }
}

/// Create or update a price source.
pub async fn upsert_price_source(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(principal): Extension<Principal>,
    payload: Result<Json<UpsertPriceSourceRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return validation(&correlation.0, rejection.body_text()),
    };
    let Some(actor) = actor(&principal) else {
        return missing_actor(&correlation.0);
    };
    match state
        .admin
        .upsert_price_source(
            &actor,
            UpsertPriceSourceCommand {
                id: request.id,
                name: request.name,
                priority: request.priority,
                licence_ref: request.licence_ref,
                enabled: request.enabled,
            },
            Some(&correlation.0),
        )
        .await
    {
        Ok(source) => Json(PriceSourceDto::from(&source)).into_response(),
        Err(error) => admin_error(error, &correlation.0),
    }
}

/// Enable or disable a price source.
pub async fn set_price_source_enabled(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(principal): Extension<Principal>,
    Path(name): Path<String>,
    payload: Result<Json<SetEnabledRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return validation(&correlation.0, rejection.body_text()),
    };
    let Some(actor) = actor(&principal) else {
        return missing_actor(&correlation.0);
    };
    match state
        .admin
        .set_price_source_enabled(&actor, &name, request.enabled, Some(&correlation.0))
        .await
    {
        Ok(source) => Json(PriceSourceDto::from(&source)).into_response(),
        Err(error) => admin_error(error, &correlation.0),
    }
}

/// Resolve a quarantined observation.
pub async fn resolve_observation(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    payload: Result<Json<ResolveObservationRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return validation(&correlation.0, rejection.body_text()),
    };
    let Some(actor) = actor(&principal) else {
        return missing_actor(&correlation.0);
    };
    match state
        .admin
        .resolve_observation(
            &actor,
            ResolveObservationCommand {
                observation_id: id,
                asset_id: request.asset_id,
            },
            Some(&correlation.0),
        )
        .await
    {
        Ok(observation) => Json(ObservationDto::from(&observation)).into_response(),
        Err(error) => admin_error(error, &correlation.0),
    }
}

/// Ignore a quarantined observation.
pub async fn ignore_observation(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    payload: Result<Json<IgnoreObservationRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return validation(&correlation.0, rejection.body_text()),
    };
    let Some(actor) = actor(&principal) else {
        return missing_actor(&correlation.0);
    };
    match state
        .admin
        .ignore_observation(
            &actor,
            IgnoreObservationCommand {
                observation_id: id,
                reason: request.reason,
            },
            Some(&correlation.0),
        )
        .await
    {
        Ok(observation) => Json(ObservationDto::from(&observation)).into_response(),
        Err(error) => admin_error(error, &correlation.0),
    }
}

/// Undeclared route: deny by default.
pub async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    not_found(
        &correlation.0,
        "the requested route is not declared in the registry API",
    )
}

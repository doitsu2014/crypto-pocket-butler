//! HTTP boundary tests: DTO mapping, the RFC 7807 problem shape, and the metrics
//! exposition. They exercise the boundary without a database or a live service.

use chrono::{DateTime, TimeZone, Utc};
use portfolio_application::common::{AppError, FieldError};
use portfolio_application::ports::ValuationRun;
use portfolio_application::privacy::PrivacyExport;
use portfolio_application::run::RunReport;
use portfolio_application::valuation::{SnapshotDetail, SnapshotView, ValuationRead};
use portfolio_domain::{
    compute_valuation, AccountId, DataStatus, Freshness, Holding, OrderSide, Portfolio,
    PortfolioAccount, PortfolioAccountId, PortfolioId, PriceSource, Quote, UserId,
    ValuationSnapshot, ValuationSnapshotId,
};
use portfolio_http::dto::{
    MembershipDto, OrderIntentRequestDto, PortfolioDto, PortfolioValuationDto, PrivacyExportDto,
    RunResultDto, SnapshotDetailDto, SnapshotDto, TargetInputDto, TargetSetRequest,
    TradeRequestDto, ValuationLineDto,
};
use portfolio_http::metrics::Metrics;
use portfolio_http::problem::{from_app_error, ProblemDetails};
use rust_decimal::Decimal;
use std::str::FromStr;

fn d(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn user() -> UserId {
    UserId::parse("11111111-1111-4111-8111-111111111111").unwrap()
}

fn portfolio() -> Portfolio {
    Portfolio::new(
        PortfolioId::generate(),
        user(),
        "My portfolio",
        "USD",
        at(0),
    )
    .unwrap()
}

fn read(valuation_portfolio: &Portfolio) -> ValuationRead {
    let holding = Holding::new(Some(AccountId::generate()), "a", d("1"), at(0)).unwrap();
    let quote = Quote::new(
        "a",
        Some(d("100")),
        Some("USD".to_string()),
        Some("test".to_string()),
        Some(at(0)),
        Freshness::Live,
    )
    .unwrap();
    let valuation = compute_valuation(
        "USD",
        std::slice::from_ref(&holding),
        std::slice::from_ref(&quote),
        &[],
        &[],
        true,
    );
    ValuationRead {
        portfolio: valuation_portfolio.clone(),
        valuation,
        as_of: at(0),
        served_from_snapshot: false,
        digest_mismatch: false,
        targets_configured: false,
        prompt: Some(portfolio_application::valuation::SET_TARGETS_PROMPT.to_string()),
    }
}

#[test]
fn portfolio_dto_maps_the_contract_fields() {
    let dto = PortfolioDto::from(&portfolio());
    assert_eq!(dto.name, "My portfolio");
    assert_eq!(dto.base_currency, "USD");
    assert_eq!(dto.id.len(), 36);
}

#[test]
fn valuation_dto_carries_the_total_and_provenance() {
    let portfolio = portfolio();
    let dto = PortfolioValuationDto::from(&read(&portfolio));
    assert_eq!(dto.portfolio_id, portfolio.id().to_string());
    assert_eq!(dto.status, "complete");
    assert_eq!(dto.total_value.as_deref(), Some("100"));
    assert_eq!(dto.lines.len(), 1);
    assert_eq!(dto.lines[0].price_source.as_deref(), Some("test"));
    assert_eq!(
        dto.prompt.as_deref(),
        Some("Set asset targets to see drift.")
    );
}

#[test]
fn valuation_line_dto_keeps_an_unpriced_line_labelled() {
    let holding = Holding::new(Some(AccountId::generate()), "b", d("3"), at(0)).unwrap();
    let valuation = compute_valuation("USD", std::slice::from_ref(&holding), &[], &[], &[], true);
    let line = ValuationLineDto::from(&valuation.lines()[0]);
    assert!(line.value.is_none());
    assert_eq!(line.data_status, DataStatus::Unpriced.as_str());
    assert_eq!(line.quantity, "3");
}

#[test]
fn target_request_maps_to_application_inputs() {
    let request = TargetSetRequest {
        targets: vec![TargetInputDto {
            asset_id: "a".to_string(),
            target_percent: d("60"),
            guardrail_points: Some(d("5")),
        }],
    };
    let inputs = request.into_inputs();
    assert_eq!(inputs[0].target_percent, d("60"));
    assert_eq!(inputs[0].guardrail_points, Some(d("5")));
}

#[test]
fn target_request_refuses_a_non_decimal_percent() {
    let value = serde_json::json!({
        "targets": [{ "assetId": "a", "targetPercent": "abc" }]
    });
    assert!(serde_json::from_value::<TargetSetRequest>(value).is_err());
}

#[test]
fn percentages_serialize_as_json_numbers_not_strings() {
    let preview = portfolio_http::dto::TargetPreviewDto {
        sum: d("90.5"),
        difference: d("9.5"),
        saved: false,
    };
    let value = serde_json::to_value(&preview).unwrap();
    assert_eq!(value["sum"], serde_json::json!(90.5));
    assert_eq!(value["difference"], serde_json::json!(9.5));
}

#[test]
fn order_intent_request_maps_its_side_and_quantity() {
    let request = OrderIntentRequestDto {
        asset_id: "a".to_string(),
        side: "sell".to_string(),
        quantity: "2.5".to_string(),
    };
    let mapped = request.into_request().unwrap();
    assert_eq!(mapped.side, OrderSide::Sell);
    assert_eq!(mapped.quantity, d("2.5"));
}

#[test]
fn order_intent_request_refuses_an_unknown_side() {
    let request = OrderIntentRequestDto {
        asset_id: "a".to_string(),
        side: "hold".to_string(),
        quantity: "1".to_string(),
    };
    assert!(request.into_request().is_err());
}

#[test]
fn trade_request_maps_price_and_source() {
    let request = TradeRequestDto {
        asset_id: "a".to_string(),
        side: "buy".to_string(),
        quantity: "1".to_string(),
        price: Some("100".to_string()),
        price_source: "user".to_string(),
        executed_at: at(0),
        idempotency_key: "idem-1".to_string(),
    };
    let mapped = request.into_request().unwrap();
    assert_eq!(mapped.price, Some(d("100")));
    assert_eq!(mapped.price_source, PriceSource::User);
}

#[test]
fn trade_request_allows_a_missing_price_for_the_use_case_to_refuse() {
    let request = TradeRequestDto {
        asset_id: "a".to_string(),
        side: "buy".to_string(),
        quantity: "1".to_string(),
        price: None,
        price_source: "venue".to_string(),
        executed_at: at(0),
        idempotency_key: "idem-1".to_string(),
    };
    assert!(request.into_request().unwrap().price.is_none());
}

#[test]
fn trade_request_refuses_a_junk_quantity() {
    let request = TradeRequestDto {
        asset_id: "a".to_string(),
        side: "buy".to_string(),
        quantity: "one".to_string(),
        price: Some("100".to_string()),
        price_source: "user".to_string(),
        executed_at: at(0),
        idempotency_key: "idem-1".to_string(),
    };
    assert!(request.into_request().is_err());
}

#[test]
fn snapshot_dto_reports_its_verification() {
    let snapshot = ValuationSnapshot::new(
        ValuationSnapshotId::generate(),
        portfolio().id(),
        at(1),
        d("100"),
        "USD",
        "a".repeat(64),
        "{}".to_string(),
        "{}".to_string(),
    )
    .unwrap();
    let dto = SnapshotDto::from(&SnapshotView { snapshot });
    assert_eq!(dto.total_value, "100");
    assert_eq!(dto.inputs_digest.len(), 64);
}

#[test]
fn snapshot_detail_dto_reports_its_verification() {
    let snapshot = ValuationSnapshot::new(
        ValuationSnapshotId::generate(),
        portfolio().id(),
        at(1),
        d("100"),
        "USD",
        "a".repeat(64),
        "{}".to_string(),
        "{}".to_string(),
    )
    .unwrap();
    let dto = SnapshotDetailDto::from(&SnapshotDetail {
        snapshot,
        verified: true,
    });
    assert!(dto.verified);
}

#[test]
fn run_result_dto_reports_failure_when_a_portfolio_failed() {
    let report = RunReport {
        portfolios: 2,
        snapshots: 1,
        skipped: 0,
        failed: 1,
        guardrail_alerts_failed: 0,
        overlays_expired: 1,
        started_at: Some(at(0)),
        duration_seconds: 3,
    };
    let dto = RunResultDto::from_report(&report);
    assert_eq!(dto.status, "failed");
    assert_eq!(dto.failed, 1);
    assert_eq!(dto.overlays_expired, 1);
}

#[test]
fn membership_dto_maps_the_row() {
    let row = PortfolioAccount::new(
        PortfolioAccountId::generate(),
        portfolio().id(),
        AccountId::generate(),
        at(0),
    );
    let dto = MembershipDto::from(&row);
    assert_eq!(dto.portfolio_id, row.portfolio_id().to_string());
    assert_eq!(dto.account_id, row.account_id().to_string());
}

#[test]
fn privacy_export_dto_maps_every_category() {
    let export = PrivacyExport {
        user_id: user(),
        portfolios: vec![portfolio()],
        memberships: vec![],
        targets: vec![],
        guardrails: vec![],
        snapshots: vec![],
        overlays: vec![],
        intents: vec![],
        trades: vec![],
    };
    let dto = PrivacyExportDto::from(export);
    assert_eq!(dto.user_id, user().to_string());
    assert_eq!(dto.portfolios.len(), 1);
}

#[test]
fn a_validation_problem_carries_the_field_paths_not_the_values() {
    let problem = from_app_error(
        &AppError::Validation(vec![FieldError::new("targetPercent", "is out of range")]),
        "corr-1",
    );
    assert_eq!(problem.status, 422);
    let errors = problem.errors.unwrap();
    assert_eq!(errors[0].field, "targetPercent");
    assert_eq!(problem.correlation_id, "corr-1");
}

#[test]
fn a_forbidden_problem_maps_to_403() {
    let problem = from_app_error(&AppError::Forbidden("owner mismatch".to_string()), "corr-1");
    assert_eq!(problem.status, 403);
}

#[test]
fn a_conflict_problem_maps_to_409() {
    let problem = from_app_error(
        &AppError::Conflict("already cancelled".to_string()),
        "corr-1",
    );
    assert_eq!(problem.status, 409);
}

#[test]
fn an_internal_problem_does_not_leak_the_detail() {
    let problem = from_app_error(&AppError::Internal("secret detail".to_string()), "corr-1");
    assert_eq!(problem.status, 500);
    assert!(problem.detail.is_none());
}

#[test]
fn a_not_found_problem_names_the_resource() {
    let problem = ProblemDetails::not_found("corr-1", "portfolio was not found");
    assert_eq!(problem.status, 404);
    assert!(problem.detail.unwrap().contains("portfolio"));
}

#[test]
fn metrics_render_the_portfolio_names() {
    let metrics = Metrics::default();
    metrics.record(200);
    metrics.record(503);
    metrics.record_valuation("partial", 2);
    metrics.record_snapshot();
    metrics.record_digest_mismatch();
    metrics.record_guardrail_alert_failure(1);
    metrics.record_run(false, 7);
    let output = metrics.render("portfolio-api");
    assert!(output.contains("portfolio_valuations_total{status=\"partial\"} 1"));
    assert!(output.contains("portfolio_unpriced_assets 2"));
    assert!(output.contains("portfolio_snapshots_total 1"));
    assert!(output.contains("portfolio_snapshot_digest_mismatch_total 1"));
    assert!(output.contains("portfolio_guardrail_alerts_failed_total 1"));
    assert!(output.contains("portfolio_valuation_run_duration_seconds 7"));
    assert!(output.contains("portfolio_health{service=\"portfolio-api\"} 1"));
    assert_eq!(metrics.errors_total(), 1);
}

#[test]
fn a_ledger_run_shape_is_constructible_for_the_worker_tests() {
    let run = ValuationRun {
        id: uuid::Uuid::new_v4(),
        job: portfolio_application::run::RUN_JOB.to_string(),
        owner: "test".to_string(),
        started_at: at(0),
        lease_until: at(0),
        finished_at: None,
        status: portfolio_application::ports::RunStatus::Running,
        detail: None,
    };
    assert_eq!(run.job, "portfolio-valuation");
}

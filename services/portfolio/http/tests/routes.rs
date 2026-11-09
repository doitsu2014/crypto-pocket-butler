//! Router tests: the public probes, the problem envelope, correlation-id
//! validation, the security headers, and the 401 path. The services are wired to
//! a `Never` fake that panics if a request reaches one, so these tests prove the
//! middleware rules without a database.

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use portfolio_application::common::VerifiedClaims;
use portfolio_application::guardrails::GuardrailService;
use portfolio_application::intents::IntentService;
use portfolio_application::portfolios::PortfolioService;
use portfolio_application::ports::{
    AnonymisationRecord, AuditClient, AuthError, Clock, ExternalError, GuardrailRepository,
    HoldingObservation, HoldingsClient, IdGenerator, IntentRepository, LedgerClient,
    LedgerTradeCommand, MarketDataClient, MembershipRepository, NotificationClient,
    OverlayRepository, PortfolioRepository, PrivacyRepository, ReadinessProbe, RegisteredService,
    RepositoryError, RunRepository, ServiceTokenRegistry, SnapshotRepository, TargetRepository,
    TokenValidator, TradeRepository, UserPreferenceClient, ValuationRun, VenueTradeObservation,
};
use portfolio_application::privacy::PrivacyService;
use portfolio_application::run::ValuationRunService;
use portfolio_application::targets::TargetService;
use portfolio_application::trades::TradeService;
use portfolio_application::trail::Trail;
use portfolio_application::valuation::ValuationService;
use portfolio_domain::{
    AuditRecord, FxRate, Guardrail, OrderIntent, OrderIntentId, Portfolio, PortfolioAccount,
    PortfolioId, Quote, RecordedTrade, RecordedTradeId, TargetAllocation, TradeOverlay, UserId,
    ValuationSnapshot,
};
use portfolio_http::metrics::Metrics;
use portfolio_http::state::AppState;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap()
}

/// A port implementation that is never reached by the tests in this file.
struct Never;

#[async_trait]
impl PortfolioRepository for Never {
    async fn insert(&self, _: &Portfolio) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: PortfolioId) -> Result<Option<Portfolio>, RepositoryError> {
        unimplemented!()
    }
    async fn find_for_user(&self, _: UserId) -> Result<Vec<Portfolio>, RepositoryError> {
        unimplemented!()
    }
    async fn update(&self, _: &Portfolio) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn anonymise_for_user(
        &self,
        _: UserId,
        _: UserId,
        _: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        unimplemented!()
    }
    async fn list_page(&self, _: i64, _: i64) -> Result<Vec<Portfolio>, RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl MembershipRepository for Never {
    async fn assign(&self, _: &PortfolioAccount) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn list_for_portfolio(
        &self,
        _: PortfolioId,
    ) -> Result<Vec<PortfolioAccount>, RepositoryError> {
        unimplemented!()
    }
    async fn list_for_user(&self, _: UserId) -> Result<Vec<PortfolioAccount>, RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl TargetRepository for Never {
    async fn list_targets(&self, _: PortfolioId) -> Result<Vec<TargetAllocation>, RepositoryError> {
        unimplemented!()
    }
    async fn list_guardrails(&self, _: PortfolioId) -> Result<Vec<Guardrail>, RepositoryError> {
        unimplemented!()
    }
    async fn replace_for_portfolio(
        &self,
        _: PortfolioId,
        _: &[TargetAllocation],
        _: &[Guardrail],
    ) -> Result<(), RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl GuardrailRepository for Never {
    async fn update(&self, _: &Guardrail) -> Result<(), RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl SnapshotRepository for Never {
    async fn insert(&self, _: &ValuationSnapshot) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn latest(&self, _: PortfolioId) -> Result<Option<ValuationSnapshot>, RepositoryError> {
        unimplemented!()
    }
    async fn list(
        &self,
        _: PortfolioId,
        _: Option<DateTime<Utc>>,
        _: i64,
        _: Option<DateTime<Utc>>,
    ) -> Result<Vec<ValuationSnapshot>, RepositoryError> {
        unimplemented!()
    }
    async fn find_by_id(
        &self,
        _: portfolio_domain::ValuationSnapshotId,
    ) -> Result<Option<ValuationSnapshot>, RepositoryError> {
        unimplemented!()
    }
    async fn exists_for_hour(
        &self,
        _: PortfolioId,
        _: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        unimplemented!()
    }
    async fn list_for_user(&self, _: UserId) -> Result<Vec<ValuationSnapshot>, RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl OverlayRepository for Never {
    async fn insert(&self, _: &TradeOverlay) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn list_for_portfolio(
        &self,
        _: PortfolioId,
    ) -> Result<Vec<TradeOverlay>, RepositoryError> {
        unimplemented!()
    }
    async fn delete_for_recorded_trade(&self, _: RecordedTradeId) -> Result<u64, RepositoryError> {
        unimplemented!()
    }
    async fn expire_before(&self, _: DateTime<Utc>) -> Result<u64, RepositoryError> {
        unimplemented!()
    }
    async fn list_for_user(&self, _: UserId) -> Result<Vec<TradeOverlay>, RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl IntentRepository for Never {
    async fn insert(&self, _: &OrderIntent) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: OrderIntentId) -> Result<Option<OrderIntent>, RepositoryError> {
        unimplemented!()
    }
    async fn list_for_portfolio(
        &self,
        _: PortfolioId,
    ) -> Result<Vec<OrderIntent>, RepositoryError> {
        unimplemented!()
    }
    async fn update(&self, _: &OrderIntent) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn list_for_user(&self, _: UserId) -> Result<Vec<OrderIntent>, RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl TradeRepository for Never {
    async fn insert_with_overlay(
        &self,
        _: &RecordedTrade,
        _: &TradeOverlay,
    ) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn find_by_idempotency_key(
        &self,
        _: PortfolioId,
        _: &str,
    ) -> Result<Option<RecordedTrade>, RepositoryError> {
        unimplemented!()
    }
    async fn find_by_id(
        &self,
        _: RecordedTradeId,
    ) -> Result<Option<RecordedTrade>, RepositoryError> {
        unimplemented!()
    }
    async fn list_for_portfolio(
        &self,
        _: PortfolioId,
    ) -> Result<Vec<RecordedTrade>, RepositoryError> {
        unimplemented!()
    }
    async fn list_for_user(&self, _: UserId) -> Result<Vec<RecordedTrade>, RepositoryError> {
        unimplemented!()
    }
    async fn anonymise_for_user(&self, _: UserId, _: UserId) -> Result<u64, RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl RunRepository for Never {
    async fn claim(&self, _: &ValuationRun) -> Result<bool, RepositoryError> {
        unimplemented!()
    }
    async fn renew(
        &self,
        _: Uuid,
        _: DateTime<Utc>,
        _: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        unimplemented!()
    }
    async fn finish(&self, _: &ValuationRun) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn sweep_expired(&self, _: DateTime<Utc>) -> Result<Vec<ValuationRun>, RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl PrivacyRepository for Never {
    async fn find_anonymisation(
        &self,
        _: &str,
    ) -> Result<Option<AnonymisationRecord>, RepositoryError> {
        unimplemented!()
    }
    async fn record_anonymisation(&self, _: &AnonymisationRecord) -> Result<(), RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl HoldingsClient for Never {
    async fn holdings_for_user(&self, _: UserId) -> Result<Vec<HoldingObservation>, ExternalError> {
        unimplemented!()
    }
    async fn transactions_for_account(
        &self,
        _: portfolio_domain::AccountId,
        _: Option<DateTime<Utc>>,
    ) -> Result<Vec<VenueTradeObservation>, ExternalError> {
        unimplemented!()
    }
}

#[async_trait]
impl MarketDataClient for Never {
    async fn prices(&self, _: &[String]) -> Result<Vec<Quote>, ExternalError> {
        unimplemented!()
    }
    async fn fx(&self, _: &str, _: &str) -> Result<FxRate, ExternalError> {
        unimplemented!()
    }
}

#[async_trait]
impl LedgerClient for Never {
    async fn record_trade(
        &self,
        _: UserId,
        _: &LedgerTradeCommand,
        _: &str,
    ) -> Result<String, ExternalError> {
        unimplemented!()
    }
}

#[async_trait]
impl NotificationClient for Never {
    async fn notify(&self, _: UserId, _: &str, _: &str, _: Value) -> Result<(), ExternalError> {
        unimplemented!()
    }
}

#[async_trait]
impl AuditClient for Never {
    async fn append(&self, _: &AuditRecord, _: &str) -> Result<(), RepositoryError> {
        unimplemented!()
    }
}

#[async_trait]
impl UserPreferenceClient for Never {
    async fn base_currency(&self, _: UserId) -> Result<Option<String>, ExternalError> {
        unimplemented!()
    }
}

impl Clock for Never {
    fn now(&self) -> DateTime<Utc> {
        at()
    }
}

impl IdGenerator for Never {
    fn next(&self) -> Uuid {
        Uuid::nil()
    }
}

#[async_trait]
impl TokenValidator for Never {
    async fn validate(&self, _: &str) -> Result<VerifiedClaims, AuthError> {
        unimplemented!()
    }
}

#[async_trait]
impl ServiceTokenRegistry for Never {
    async fn service_for(&self, _: &str) -> Result<Option<RegisteredService>, RepositoryError> {
        unimplemented!()
    }
    fn token_for(&self, _: &str) -> Option<String> {
        None
    }
}

#[async_trait]
impl ReadinessProbe for Never {
    async fn check(&self) -> Result<(), String> {
        Ok(())
    }
}

fn state() -> AppState {
    let clock: std::sync::Arc<dyn Clock> = std::sync::Arc::new(Never);
    let ids: std::sync::Arc<dyn IdGenerator> = std::sync::Arc::new(Never);
    let portfolios_repo: std::sync::Arc<dyn PortfolioRepository> = std::sync::Arc::new(Never);
    let memberships: std::sync::Arc<dyn MembershipRepository> = std::sync::Arc::new(Never);
    let targets: std::sync::Arc<dyn TargetRepository> = std::sync::Arc::new(Never);
    let guardrails_repo: std::sync::Arc<dyn GuardrailRepository> = std::sync::Arc::new(Never);
    let snapshots: std::sync::Arc<dyn SnapshotRepository> = std::sync::Arc::new(Never);
    let overlays: std::sync::Arc<dyn OverlayRepository> = std::sync::Arc::new(Never);
    let intents_repo: std::sync::Arc<dyn IntentRepository> = std::sync::Arc::new(Never);
    let trades_repo: std::sync::Arc<dyn TradeRepository> = std::sync::Arc::new(Never);
    let _runs: std::sync::Arc<dyn RunRepository> = std::sync::Arc::new(Never);
    let privacy_repo: std::sync::Arc<dyn PrivacyRepository> = std::sync::Arc::new(Never);
    let holdings: std::sync::Arc<dyn HoldingsClient> = std::sync::Arc::new(Never);
    let market: std::sync::Arc<dyn MarketDataClient> = std::sync::Arc::new(Never);
    let ledger: std::sync::Arc<dyn LedgerClient> = std::sync::Arc::new(Never);
    let notifications: std::sync::Arc<dyn NotificationClient> = std::sync::Arc::new(Never);
    let audit: std::sync::Arc<dyn AuditClient> = std::sync::Arc::new(Never);
    let preferences: std::sync::Arc<dyn UserPreferenceClient> = std::sync::Arc::new(Never);

    let trail = std::sync::Arc::new(Trail::new(audit, clock.clone()));
    let valuation = std::sync::Arc::new(ValuationService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        targets.clone(),
        snapshots,
        overlays,
        holdings,
        market.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let portfolios = std::sync::Arc::new(PortfolioService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        preferences,
        clock.clone(),
        ids.clone(),
    ));
    let targets_service = std::sync::Arc::new(TargetService::new(
        portfolios_repo.clone(),
        targets,
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let intents = std::sync::Arc::new(IntentService::new(
        portfolios_repo.clone(),
        intents_repo.clone(),
        market,
        valuation.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let trades = std::sync::Arc::new(TradeService::new(
        portfolios_repo.clone(),
        trades_repo.clone(),
        ledger,
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let guardrails = std::sync::Arc::new(GuardrailService::new(
        std::sync::Arc::new(Never),
        guardrails_repo,
        notifications,
        clock.clone(),
    ));
    let privacy = std::sync::Arc::new(PrivacyService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        std::sync::Arc::new(Never),
        std::sync::Arc::new(Never),
        std::sync::Arc::new(Never),
        intents_repo,
        trades_repo.clone(),
        privacy_repo,
        trail,
        clock.clone(),
        ids.clone(),
    ));
    let _ = ValuationRunService::new(
        portfolios_repo,
        memberships,
        std::sync::Arc::new(Never),
        std::sync::Arc::new(Never),
        trades_repo,
        std::sync::Arc::new(Never),
        valuation.clone(),
        guardrails.clone(),
        clock,
        ids,
    );

    AppState {
        service_name: "portfolio-api",
        portfolios,
        valuation,
        targets: targets_service,
        intents,
        trades,
        guardrails,
        privacy,
        token_validator: std::sync::Arc::new(Never),
        service_tokens: std::sync::Arc::new(Never),
        readiness: std::sync::Arc::new(Never),
        metrics: std::sync::Arc::new(Metrics::default()),
    }
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn health_is_public() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_text(response).await.contains("portfolio-api"));
}

#[tokio::test]
async fn readiness_is_public_and_ok() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_are_public() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_text(response).await.contains("portfolio_health"));
}

#[tokio::test]
async fn a_protected_route_without_a_token_is_401() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/v1/portfolios")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let body = body_text(response).await;
    assert!(body.contains("\"correlationId\":"));
}

#[tokio::test]
async fn the_snapshot_route_also_requires_an_identity() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/v1/portfolios/11111111-1111-4111-8111-111111111111/snapshots")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_smoke_route_requires_a_service_token() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/v1/internal/smoke")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_unknown_route_is_404() {
    let response = portfolio_http::router(state())
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_invalid_correlation_id_is_400() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .header("x-correlation-id", "not valid!")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn security_headers_are_present() {
    let response = portfolio_http::router(state())
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response
        .headers()
        .get(header::STRICT_TRANSPORT_SECURITY)
        .is_some());
    assert!(response
        .headers()
        .get(header::X_CONTENT_TYPE_OPTIONS)
        .is_some());
    assert!(response.headers().get(header::X_FRAME_OPTIONS).is_some());
}

#[allow(dead_code)]
fn _assert_claims(_: VerifiedClaims) {}

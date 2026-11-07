//! HTTP boundary tests over in-memory ports. No database, no network.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use analytics_application::ports::{
    AuditClient, AuditRecord, AuthError, Clock, ConsentChecker, ConsentRepository, EventRepository,
    ExternalError, FlagRepository, Pause, Principal, ReadinessProbe, RepositoryError, Role,
    SaltProvider, SaltVersion, ServiceCaller, ServiceTokenRegistry, TokenValidator,
};
use analytics_application::{
    AggregateService, EraseService, FlagService, IngestEvent, RetentionService, Trail,
};
use analytics_domain::{
    sanitize_event, AnalyticsEvent, AnalyticsEventId, ConsentCheck, FeatureFlag, FlagStatus,
    FlagUpdate, Metric, MetricAggregate, Period,
};
use analytics_http::{api_router, worker_router, AppState};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Duration, TimeZone, Utc};
use tower::ServiceExt;

const SUBJECT: &str = "11111111-1111-4111-8111-111111111111";
const ADMIN: &str = "22222222-2222-4222-8222-222222222222";
const OTHER: &str = "33333333-3333-4333-8333-333333333333";
const ERASE_USER: &str = "44444444-4444-4444-8444-444444444444";

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[derive(Debug)]
struct MutableClock {
    now: Mutex<DateTime<Utc>>,
}

impl MutableClock {
    fn at(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    fn advance(&self, duration: Duration) {
        *self.now.lock().unwrap() += duration;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

struct NoPause;

#[async_trait]
impl Pause for NoPause {
    async fn pause(&self) {}
}

#[derive(Default)]
struct FakeEvents {
    stored: Mutex<Vec<AnalyticsEvent>>,
    consents: Mutex<Vec<ConsentCheck>>,
}

#[async_trait]
impl EventRepository for FakeEvents {
    async fn insert(
        &self,
        event: &AnalyticsEvent,
        consent: Option<&ConsentCheck>,
    ) -> Result<(), RepositoryError> {
        self.stored.lock().unwrap().push(event.clone());
        if let Some(consent) = consent {
            self.consents.lock().unwrap().push(consent.clone());
        }
        Ok(())
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AnalyticsEvent>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.occurred_at() >= from && event.occurred_at() < to)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        let mut removed = 0u64;
        stored.retain(|event| {
            if removed < u64::from(limit) && event.occurred_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }

    async fn delete_by_user_refs(&self, _user_refs: &[String]) -> Result<u64, RepositoryError> {
        Ok(0)
    }
}

#[derive(Default)]
struct FakeConsents {
    stored: Mutex<Vec<ConsentCheck>>,
}

#[async_trait]
impl ConsentRepository for FakeConsents {
    async fn insert(&self, check: &ConsentCheck) -> Result<(), RepositoryError> {
        self.stored.lock().unwrap().push(check.clone());
        Ok(())
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        let mut removed = 0u64;
        stored.retain(|check| {
            if removed < u64::from(limit) && check.checked_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }

    async fn delete_by_user_refs(&self, user_refs: &[String]) -> Result<u64, RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        let mut removed = 0u64;
        stored.retain(|check| {
            let matches = check
                .user_ref()
                .is_some_and(|user_ref| user_refs.iter().any(|value| value == user_ref));
            if matches {
                removed += 1;
            }
            !matches
        });
        Ok(removed)
    }
}

struct FakeConsentChecker {
    result: Mutex<Result<bool, ExternalError>>,
}

#[async_trait]
impl ConsentChecker for FakeConsentChecker {
    async fn analytics_granted(&self, _user_id: &str) -> Result<bool, ExternalError> {
        self.result.lock().unwrap().clone()
    }
}

struct FakeSalts {
    current: SaltVersion,
    retained: Vec<SaltVersion>,
}

#[async_trait]
impl SaltProvider for FakeSalts {
    async fn current(&self) -> Result<SaltVersion, ExternalError> {
        Ok(self.current.clone())
    }

    async fn retained(&self) -> Result<Vec<SaltVersion>, ExternalError> {
        Ok(self.retained.clone())
    }
}

fn salt(version: &str) -> SaltVersion {
    SaltVersion {
        version: version.to_string(),
        salt: vec![1u8; 32],
    }
}

#[derive(Default)]
struct FakeAudit {
    records: Mutex<Vec<AuditRecord>>,
}

#[async_trait]
impl AuditClient for FakeAudit {
    async fn append(&self, record: &AuditRecord) -> Result<(), ExternalError> {
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
}

#[derive(Default)]
struct FakeFlags {
    flags: Mutex<Vec<FeatureFlag>>,
}

#[async_trait]
impl FlagRepository for FakeFlags {
    async fn insert(&self, flag: &FeatureFlag) -> Result<(), RepositoryError> {
        let mut flags = self.flags.lock().unwrap();
        if flags.iter().any(|existing| existing.key() == flag.key()) {
            return Err(RepositoryError::Conflict(
                "the key already exists".to_string(),
            ));
        }
        flags.push(flag.clone());
        Ok(())
    }

    async fn update(&self, flag: &FeatureFlag) -> Result<(), RepositoryError> {
        let mut flags = self.flags.lock().unwrap();
        if let Some(existing) = flags
            .iter_mut()
            .find(|existing| existing.key() == flag.key())
        {
            *existing = flag.clone();
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, RepositoryError> {
        let mut flags = self.flags.lock().unwrap();
        let before = flags.len();
        flags.retain(|flag| flag.key() != key);
        Ok(flags.len() != before)
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<FeatureFlag>, RepositoryError> {
        Ok(self
            .flags
            .lock()
            .unwrap()
            .iter()
            .find(|flag| flag.key() == key)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<FeatureFlag>, RepositoryError> {
        Ok(self.flags.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct FakeAggregates {
    stored: Mutex<Vec<MetricAggregate>>,
}

#[async_trait]
impl analytics_application::ports::AggregateRepository for FakeAggregates {
    async fn upsert(&self, aggregate: &MetricAggregate) -> Result<(), RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        if let Some(existing) = stored.iter_mut().find(|existing| {
            existing.metric() == aggregate.metric()
                && existing.period() == aggregate.period()
                && existing.cohort() == aggregate.cohort()
                && existing.period_start() == aggregate.period_start()
        }) {
            *existing = aggregate.clone();
        } else {
            stored.push(aggregate.clone());
        }
        Ok(())
    }

    async fn list(
        &self,
        metric: Option<Metric>,
        period: Option<Period>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<MetricAggregate>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|aggregate| metric.is_none_or(|value| aggregate.metric() == value))
            .filter(|aggregate| period.is_none_or(|value| aggregate.period() == value))
            .filter(|aggregate| from.is_none_or(|value| aggregate.period_start() >= value))
            .filter(|aggregate| to.is_none_or(|value| aggregate.period_start() < value))
            .take(limit as usize)
            .cloned()
            .collect())
    }
}

struct FakeTokens {
    entries: Vec<(String, Result<Principal, AuthError>)>,
}

#[async_trait]
impl TokenValidator for FakeTokens {
    async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == token)
            .map(|(_, result)| result.clone())
            .unwrap_or_else(|| Err(AuthError::Invalid("unknown token".to_string())))
    }
}

struct FakeServiceTokens {
    entries: Vec<(String, ServiceCaller)>,
}

#[async_trait]
impl ServiceTokenRegistry for FakeServiceTokens {
    async fn resolve(&self, token: &str) -> Result<Option<ServiceCaller>, RepositoryError> {
        Ok(self
            .entries
            .iter()
            .find(|(candidate, _)| candidate == token)
            .map(|(_, caller)| caller.clone()))
    }
}

struct Ready(bool);

#[async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        if self.0 {
            Ok(())
        } else {
            Err("a dependency is down".to_string())
        }
    }
}

struct Harness {
    state: AppState,
    clock: Arc<MutableClock>,
    events: Arc<FakeEvents>,
}

fn harness() -> Harness {
    harness_with(Ok(true), true)
}

fn harness_with(consent: Result<bool, ExternalError>, ready: bool) -> Harness {
    let events = Arc::new(FakeEvents::default());
    let consents = Arc::new(FakeConsents::default());
    let checker = Arc::new(FakeConsentChecker {
        result: Mutex::new(consent),
    });
    let salts = Arc::new(FakeSalts {
        current: salt("v1"),
        retained: vec![salt("v1"), salt("v2")],
    });
    let audit = Arc::new(FakeAudit::default());
    let flags_repo = Arc::new(FakeFlags::default());
    let aggregates_repo = Arc::new(FakeAggregates::default());
    let clock = Arc::new(MutableClock::at(at(6, 10)));
    let trail = Arc::new(Trail::new(audit, clock.clone()));

    let ingest = Arc::new(IngestEvent::new(
        events.clone(),
        checker,
        salts.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let flags = Arc::new(FlagService::new(flags_repo, clock.clone(), trail.clone()));
    let aggregates = Arc::new(AggregateService::new(
        events.clone(),
        aggregates_repo,
        clock.clone(),
    ));
    let retention = Arc::new(RetentionService::new(
        events.clone(),
        consents.clone(),
        clock.clone(),
        Arc::new(NoPause),
    ));
    let erase = Arc::new(EraseService::new(events.clone(), consents, salts, trail));

    let token_validator = Arc::new(FakeTokens {
        entries: vec![
            (
                "user-token".to_string(),
                Ok(Principal {
                    subject: SUBJECT.to_string(),
                    role: Role::User,
                }),
            ),
            (
                "other-token".to_string(),
                Ok(Principal {
                    subject: OTHER.to_string(),
                    role: Role::User,
                }),
            ),
            (
                "admin-token".to_string(),
                Ok(Principal {
                    subject: ADMIN.to_string(),
                    role: Role::Admin,
                }),
            ),
        ],
    });
    let service_tokens = Arc::new(FakeServiceTokens {
        entries: vec![
            (
                "identity-token".to_string(),
                ServiceCaller {
                    service: "identity".to_string(),
                    operations: vec!["privacy.erase".to_string()],
                },
            ),
            (
                "gateway-token".to_string(),
                ServiceCaller {
                    service: "gateway".to_string(),
                    operations: vec!["smoke".to_string()],
                },
            ),
            (
                "run-token".to_string(),
                ServiceCaller {
                    service: "job-orchestration".to_string(),
                    operations: vec!["runs".to_string()],
                },
            ),
            (
                "smoke-token".to_string(),
                ServiceCaller {
                    service: "smoke-reader".to_string(),
                    operations: vec!["smoke".to_string()],
                },
            ),
        ],
    });

    let state = AppState {
        service_name: "analytics-api",
        ingest,
        flags,
        aggregates,
        retention,
        erase,
        service_tokens,
        token_validator,
        clock: clock.clone(),
        readiness: Arc::new(Ready(ready)),
        metrics: Arc::new(analytics_http::metrics::Metrics::default()),
    };

    Harness {
        state,
        clock,
        events,
    }
}

async fn call(router: Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = router.oneshot(request).await.expect("router answers");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, value)
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn user_request(
    method: &str,
    uri: &str,
    token: &str,
    subject: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let body = body.map_or_else(Body::empty, |value| Body::from(value.to_string()));
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("x-auth-subject", subject)
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}

fn service_request(
    method: &str,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let body = body.map_or_else(Body::empty, |value| Body::from(value.to_string()));
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}

#[tokio::test]
async fn health_is_live() {
    let harness = harness();
    let (status, body) = call(api_router(harness.state), get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn readiness_reports_ready() {
    let harness = harness();
    let (status, body) = call(api_router(harness.state), get("/readyz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn readiness_returns_503_when_a_dependency_is_down() {
    let harness = harness_with(Ok(true), false);
    let (status, _) = call(api_router(harness.state), get("/readyz")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn metrics_are_exposed() {
    let harness = harness();
    let response = api_router(harness.state)
        .oneshot(get("/metrics"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("analytics_health"));
}

#[tokio::test]
async fn ingest_accepts_a_consented_event() {
    let harness = harness();
    let body = serde_json::json!({
        "eventName": "valuation_read",
        "occurredAt": "2026-11-06T09:00:00Z",
        "properties": {"freshness": "live"}
    });
    let (status, response) = call(
        api_router(harness.state),
        user_request("POST", "/v1/events", "user-token", SUBJECT, Some(body)),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(response["status"], "accepted");
    assert_eq!(harness.events.stored.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn ingest_refuses_withdrawn_consent() {
    let harness = harness_with(Ok(false), true);
    let body = serde_json::json!({
        "eventName": "valuation_read",
        "occurredAt": "2026-11-06T09:00:00Z"
    });
    let (status, _) = call(
        api_router(harness.state),
        user_request("POST", "/v1/events", "user-token", SUBJECT, Some(body)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(harness.events.stored.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ingest_rejects_an_unknown_event() {
    let harness = harness();
    let body = serde_json::json!({
        "eventName": "not_in_catalogue",
        "occurredAt": "2026-11-06T09:00:00Z"
    });
    let (status, response) = call(
        api_router(harness.state),
        user_request("POST", "/v1/events", "user-token", SUBJECT, Some(body)),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response["errors"][0]["field"], "eventName");
}

#[tokio::test]
async fn a_rejected_event_name_never_becomes_a_metric_label() {
    let harness = harness();
    let email = "person@example.com";
    let body = serde_json::json!({
        "eventName": email,
        "occurredAt": "2026-11-06T09:00:00Z"
    });
    let (status, _) = call(
        api_router(harness.state.clone()),
        user_request("POST", "/v1/events", "user-token", SUBJECT, Some(body)),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let response = api_router(harness.state)
        .oneshot(get("/metrics"))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!text.contains(email));
    assert!(text.contains("event_name=\"unknown\""));
}

#[tokio::test]
async fn ingest_requires_a_bearer_token() {
    let harness = harness();
    let request = Request::builder()
        .method("POST")
        .uri("/v1/events")
        .header("x-auth-subject", SUBJECT)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"eventName": "error_shown", "occurredAt": "2026-11-06T09:00:00Z"})
                .to_string(),
        ))
        .unwrap();
    let (status, _) = call(api_router(harness.state), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ingest_refuses_a_subject_header_mismatch() {
    let harness = harness();
    let body = serde_json::json!({
        "eventName": "error_shown",
        "occurredAt": "2026-11-06T09:00:00Z"
    });
    let (status, _) = call(
        api_router(harness.state),
        user_request("POST", "/v1/events", "user-token", OTHER, Some(body)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ingest_returns_503_when_consent_is_unavailable() {
    let harness = harness_with(Err(ExternalError::Unavailable("timeout".to_string())), true);
    let body = serde_json::json!({
        "eventName": "valuation_read",
        "occurredAt": "2026-11-06T09:00:00Z"
    });
    let (status, _) = call(
        api_router(harness.state),
        user_request("POST", "/v1/events", "user-token", SUBJECT, Some(body)),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn a_missing_flag_evaluates_off() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        user_request("GET", "/v1/flags/missing", "user-token", SUBJECT, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], false);
}

#[tokio::test]
async fn a_full_rollout_flag_evaluates_on() {
    let harness = harness();
    let create = serde_json::json!({
        "key": "new-dashboard",
        "owner": "product",
        "expiresAt": "2026-11-30T00:00:00Z",
        "rolloutPercentage": 100,
        "status": "active"
    });
    let (status, _) = call(
        api_router(harness.state.clone()),
        user_request(
            "POST",
            "/v1/admin/flags",
            "admin-token",
            ADMIN,
            Some(create),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "GET",
            "/v1/flags/new-dashboard",
            "user-token",
            SUBJECT,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], true);
}

#[tokio::test]
async fn flag_read_requires_a_token() {
    let harness = harness();
    let (status, _) = call(api_router(harness.state), get("/v1/flags/x")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn aggregates_require_the_admin_role() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        user_request(
            "GET",
            "/v1/analytics/aggregates",
            "user-token",
            SUBJECT,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn an_admin_reads_an_empty_aggregate_page() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "GET",
            "/v1/analytics/aggregates",
            "admin-token",
            ADMIN,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["aggregates"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn an_admin_creates_lists_updates_and_deletes_a_flag() {
    let harness = harness();
    let create = serde_json::json!({
        "key": "new-dashboard",
        "owner": "product",
        "expiresAt": "2026-11-30T00:00:00Z",
        "rolloutPercentage": 25,
        "status": "active"
    });
    let (status, _) = call(
        api_router(harness.state.clone()),
        user_request(
            "POST",
            "/v1/admin/flags",
            "admin-token",
            ADMIN,
            Some(create),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = call(
        api_router(harness.state.clone()),
        user_request("GET", "/v1/admin/flags", "admin-token", ADMIN, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    let patch = serde_json::json!({"rolloutPercentage": 75});
    let (status, body) = call(
        api_router(harness.state.clone()),
        user_request(
            "PATCH",
            "/v1/admin/flags/new-dashboard",
            "admin-token",
            ADMIN,
            Some(patch),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rolloutPercentage"], 75);

    let (status, _) = call(
        api_router(harness.state.clone()),
        user_request(
            "DELETE",
            "/v1/admin/flags/new-dashboard",
            "admin-token",
            ADMIN,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = call(
        api_router(harness.state),
        user_request("GET", "/v1/admin/flags", "admin-token", ADMIN, None),
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn an_admin_create_without_an_owner_is_rejected() {
    let harness = harness();
    let create = serde_json::json!({
        "key": "new-dashboard",
        "owner": "   ",
        "expiresAt": "2026-11-30T00:00:00Z",
        "rolloutPercentage": 25
    });
    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "POST",
            "/v1/admin/flags",
            "admin-token",
            ADMIN,
            Some(create),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "owner");
}

#[tokio::test]
async fn expired_flags_are_listed_for_cleanup() {
    let harness = harness();
    let create = serde_json::json!({
        "key": "short-lived",
        "owner": "product",
        "expiresAt": "2026-11-07T00:00:00Z",
        "rolloutPercentage": 10,
        "status": "active"
    });
    call(
        api_router(harness.state.clone()),
        user_request(
            "POST",
            "/v1/admin/flags",
            "admin-token",
            ADMIN,
            Some(create),
        ),
    )
    .await;
    harness.clock.advance(Duration::days(2));
    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "GET",
            "/v1/admin/flags?expired=true",
            "admin-token",
            ADMIN,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["status"], "expired");
}

#[tokio::test]
async fn identity_can_erase_a_users_events() {
    let harness = harness();
    let body = serde_json::json!({"authUserId": ERASE_USER});
    let (status, _) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/privacy/erase",
            "identity-token",
            Some(body),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn another_service_cannot_erase_events() {
    let harness = harness();
    let body = serde_json::json!({"authUserId": ERASE_USER});
    let (status, _) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/privacy/erase",
            "gateway-token",
            Some(body),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn erase_rejects_a_malformed_user_id() {
    let harness = harness();
    let body = serde_json::json!({"authUserId": "not-a-uuid"});
    let (status, _) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/privacy/erase",
            "identity-token",
            Some(body),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn smoke_requires_a_smoke_token() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state.clone()),
        service_request("GET", "/v1/internal/smoke", "smoke-token", None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        api_router(harness.state),
        service_request("GET", "/v1/internal/smoke", "gateway-token", None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn smoke_rejects_an_unknown_token() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        service_request("GET", "/v1/internal/smoke", "nope", None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_worker_runs_aggregation_for_the_job_orchestrator() {
    let harness = harness();
    let (status, body) = call(
        worker_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/internal/run/analytics-aggregate",
            "run-token",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "success");
    assert!(body["scannedDaily"].as_u64().is_some());

    let response = worker_router(harness.state)
        .oneshot(get("/metrics"))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("analytics_aggregate_duration_seconds_count"));
}

#[tokio::test]
async fn the_worker_runs_retention_for_the_job_orchestrator() {
    let harness = harness();
    let (status, body) = call(
        worker_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/run/analytics-retention",
            "run-token",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "success");
}

#[tokio::test]
async fn the_worker_rejects_an_identity_token_on_a_run_path() {
    let harness = harness();
    let (status, _) = call(
        worker_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/run/analytics-aggregate",
            "identity-token",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_worker_serves_health() {
    let harness = harness();
    let (status, body) = call(worker_router(harness.state), get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn an_unknown_path_returns_a_problem_document() {
    let harness = harness();
    let (status, body) = call(api_router(harness.state), get("/v1/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert!(body["correlationId"].as_str().is_some());
}

#[tokio::test]
async fn a_stored_flag_with_an_expired_row_evaluates_off() {
    let harness = harness();
    let mut flag = FeatureFlag::create(
        analytics_domain::FeatureFlagId::generate(),
        analytics_domain::NewFeatureFlag {
            key: "expired".to_string(),
            owner: "product".to_string(),
            expires_at: at(7, 0),
            rollout_percentage: 100,
            status: Some("active".to_string()),
        },
        at(6, 10),
    )
    .unwrap();
    flag.apply_update(
        FlagUpdate {
            status: Some(FlagStatus::Active.as_str().to_string()),
            ..FlagUpdate::default()
        },
        at(6, 10),
    )
    .unwrap();
    // A repository insert is not exposed by the service; create through the API
    // instead so the cache and audit paths are exercised.
    let create = serde_json::json!({
        "key": "expired",
        "owner": "product",
        "expiresAt": "2026-11-07T00:00:00Z",
        "rolloutPercentage": 100,
        "status": "active"
    });
    call(
        api_router(harness.state.clone()),
        user_request(
            "POST",
            "/v1/admin/flags",
            "admin-token",
            ADMIN,
            Some(create),
        ),
    )
    .await;
    harness.clock.advance(Duration::days(2));
    let (status, body) = call(
        api_router(harness.state),
        user_request("GET", "/v1/flags/expired", "user-token", SUBJECT, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], false);
}

#[tokio::test]
async fn a_consent_check_is_recorded_for_an_accepted_event() {
    let harness = harness();
    let body = serde_json::json!({
        "eventName": "valuation_read",
        "occurredAt": "2026-11-06T09:00:00Z"
    });
    call(
        api_router(harness.state),
        user_request("POST", "/v1/events", "user-token", SUBJECT, Some(body)),
    )
    .await;
    assert_eq!(harness.events.consents.lock().unwrap().len(), 1);
}

#[test]
fn the_fixture_catalogue_names_match_the_domain() {
    let sanitized = sanitize_event("error_shown", at(6, 10), BTreeMap::new()).unwrap();
    assert_eq!(sanitized.event_name(), "error_shown");
    let event = AnalyticsEvent::store(
        AnalyticsEventId::generate(),
        sanitized,
        Some("ref".to_string()),
        at(6, 10),
    );
    assert_eq!(event.event_name(), "error_shown");
}

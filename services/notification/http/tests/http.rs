//! HTTP boundary tests over in-memory ports. No database, no network.

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use notification_application::ports::{
    AlertRepository, AnonymisedRows, AttemptRepository, AuditClient, AuditRecord, AuthError, Clock,
    ContactDirectory, DeliveryControl, EmailMessage, EmailSender, ExternalError, IdGenerator,
    InsertOutcome, Pause, PreferenceRepository, Principal, ReadinessProbe, RepositoryError, Role,
    ServiceCaller, ServiceTokenRegistry, TokenValidator,
};
use notification_application::{
    DeliveryService, EraseService, FeedService, PreferenceService, RetentionService, SubmitAlert,
    Trail,
};
use notification_domain::{
    Alert, AlertId, AlertStatus, DeliveryAttempt, NotificationPreference, UserId,
};
use notification_http::{api_router, worker_router, AppState};
use tower::ServiceExt;

const USER: &str = "11111111-1111-4111-8111-111111111111";
const OTHER: &str = "22222222-2222-4222-8222-222222222222";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 8, hour, 0, 0).unwrap()
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

    fn advance(&self, duration: ChronoDuration) {
        *self.now.lock().unwrap() += duration;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

#[derive(Default)]
struct SequentialIds {
    counter: AtomicU64,
}

impl IdGenerator for SequentialIds {
    fn next(&self) -> uuid::Uuid {
        let value = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        uuid::Uuid::from_u128(0x2000_0000_0000_4000_8000_0000_0000_0000 + u128::from(value))
    }
}

struct NoPause;

#[async_trait]
impl Pause for NoPause {
    async fn pause_for(&self, _duration: Duration) {}
}

#[derive(Default)]
struct Store {
    alerts: Mutex<Vec<Alert>>,
    attempts: Mutex<Vec<DeliveryAttempt>>,
    preferences: Mutex<Vec<NotificationPreference>>,
}

struct FakeAlerts(Arc<Store>);

#[async_trait]
impl AlertRepository for FakeAlerts {
    async fn insert(&self, alert: &Alert) -> Result<InsertOutcome, RepositoryError> {
        let mut alerts = self.0.alerts.lock().unwrap();
        if let Some(existing) = alerts
            .iter()
            .find(|stored| stored.idempotency_key() == alert.idempotency_key())
        {
            return Ok(InsertOutcome::Duplicate(existing.clone()));
        }
        alerts.push(alert.clone());
        Ok(InsertOutcome::Inserted(alert.clone()))
    }

    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Alert>, RepositoryError> {
        Ok(self
            .0
            .alerts
            .lock()
            .unwrap()
            .iter()
            .find(|alert| alert.idempotency_key() == key)
            .cloned())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
        before: Option<(DateTime<Utc>, AlertId)>,
        limit: u32,
    ) -> Result<Vec<Alert>, RepositoryError> {
        let disabled: Vec<notification_domain::NotificationType> = self
            .0
            .preferences
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.channel() == notification_domain::Channel::InApp && !row.enabled())
            .map(NotificationPreference::kind)
            .collect();
        let mut alerts: Vec<Alert> = self
            .0
            .alerts
            .lock()
            .unwrap()
            .iter()
            .filter(|alert| alert.user_id() == user_id)
            // BR2.1 and WF4.2: a type disabled for in-app never reaches the feed.
            .filter(|alert| !disabled.contains(&alert.kind()))
            .cloned()
            .collect();
        alerts.sort_by(|left, right| {
            right
                .created_at()
                .cmp(&left.created_at())
                .then_with(|| right.id().as_uuid().cmp(&left.id().as_uuid()))
        });
        Ok(alerts
            .into_iter()
            .filter(|alert| match before {
                None => true,
                Some((created_at, id)) => {
                    alert.created_at() < created_at
                        || (alert.created_at() == created_at && alert.id().as_uuid() < id.as_uuid())
                }
            })
            .take(limit as usize)
            .collect())
    }

    async fn due(&self, now: DateTime<Utc>, limit: u32) -> Result<Vec<Alert>, RepositoryError> {
        Ok(self
            .0
            .alerts
            .lock()
            .unwrap()
            .iter()
            .filter(|alert| alert.status() == AlertStatus::Queued)
            .filter(|alert| alert.next_attempt_at().is_some_and(|at| at <= now))
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn has_due(&self, now: DateTime<Utc>) -> Result<bool, RepositoryError> {
        Ok(self.0.alerts.lock().unwrap().iter().any(|alert| {
            alert.status() == AlertStatus::Queued
                && alert.next_attempt_at().is_some_and(|at| at <= now)
        }))
    }

    async fn set_status(
        &self,
        id: AlertId,
        status: AlertStatus,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<(), RepositoryError> {
        let mut alerts = self.0.alerts.lock().unwrap();
        let Some(alert) = alerts.iter_mut().find(|alert| alert.id() == id) else {
            return Err(RepositoryError::Unavailable("missing alert".to_string()));
        };
        *alert = Alert::from_stored(
            alert.id(),
            alert.user_id(),
            alert.kind(),
            alert.idempotency_key(),
            alert.payload().clone(),
            status.as_str(),
            alert.sanitised_fields().to_vec(),
            alert.created_at(),
            next_attempt_at,
        )
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        Ok(())
    }

    async fn anonymise_user(
        &self,
        user_id: UserId,
        replacement: UserId,
    ) -> Result<AnonymisedRows, RepositoryError> {
        let mut alerts = self.0.alerts.lock().unwrap();
        let mut alerts_changed = 0u64;
        for alert in alerts.iter_mut().filter(|alert| alert.user_id() == user_id) {
            *alert = Alert::from_stored(
                alert.id(),
                replacement,
                alert.kind(),
                alert.idempotency_key(),
                alert.payload().clone(),
                alert.status().as_str(),
                alert.sanitised_fields().to_vec(),
                alert.created_at(),
                alert.next_attempt_at(),
            )
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
            alerts_changed += 1;
        }
        let mut attempts = self.0.attempts.lock().unwrap();
        let mut attempts_changed = 0u64;
        for attempt in attempts
            .iter_mut()
            .filter(|attempt| attempt.user_id() == user_id)
        {
            *attempt = DeliveryAttempt::from_stored(
                attempt.id(),
                attempt.alert_id(),
                replacement,
                attempt.channel(),
                attempt.attempt(),
                attempt.status().as_str(),
                attempt.error_category(),
                attempt.attempted_at(),
            )
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
            attempts_changed += 1;
        }
        Ok(AnonymisedRows {
            alerts: alerts_changed,
            attempts: attempts_changed,
        })
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let mut alerts = self.0.alerts.lock().unwrap();
        let mut removed = 0u64;
        alerts.retain(|alert| {
            if removed < u64::from(limit) && alert.created_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }
}

struct FakeAttempts(Arc<Store>);

#[async_trait]
impl AttemptRepository for FakeAttempts {
    async fn insert(&self, attempt: &DeliveryAttempt) -> Result<(), RepositoryError> {
        self.0.attempts.lock().unwrap().push(attempt.clone());
        Ok(())
    }

    async fn list_for_alert(
        &self,
        alert_id: AlertId,
    ) -> Result<Vec<DeliveryAttempt>, RepositoryError> {
        Ok(self
            .0
            .attempts
            .lock()
            .unwrap()
            .iter()
            .filter(|attempt| attempt.alert_id() == alert_id)
            .cloned()
            .collect())
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let mut attempts = self.0.attempts.lock().unwrap();
        let mut removed = 0u64;
        attempts.retain(|attempt| {
            if removed < u64::from(limit) && attempt.attempted_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }
}

struct FakePreferences(Arc<Store>);

#[async_trait]
impl PreferenceRepository for FakePreferences {
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<NotificationPreference>, RepositoryError> {
        Ok(self
            .0
            .preferences
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.user_id() == user_id)
            .cloned()
            .collect())
    }

    async fn upsert(&self, preference: &NotificationPreference) -> Result<(), RepositoryError> {
        let mut rows = self.0.preferences.lock().unwrap();
        rows.retain(|row| {
            !(row.user_id() == preference.user_id()
                && row.kind() == preference.kind()
                && row.channel() == preference.channel())
        });
        rows.push(preference.clone());
        Ok(())
    }

    async fn upsert_all(
        &self,
        preferences: &[NotificationPreference],
    ) -> Result<(), RepositoryError> {
        let mut rows = self.0.preferences.lock().unwrap();
        for preference in preferences {
            rows.retain(|row| {
                !(row.user_id() == preference.user_id()
                    && row.kind() == preference.kind()
                    && row.channel() == preference.channel())
            });
            rows.push(preference.clone());
        }
        Ok(())
    }
}

#[derive(Default)]
struct FakeControl {
    state: Mutex<(bool, Option<uuid::Uuid>)>,
}

#[async_trait]
impl DeliveryControl for FakeControl {
    async fn request_wake(&self, _at: DateTime<Utc>) -> Result<(), RepositoryError> {
        self.state.lock().unwrap().0 = true;
        Ok(())
    }

    async fn wake_requested(&self) -> Result<bool, RepositoryError> {
        Ok(self.state.lock().unwrap().0)
    }

    async fn clear_wake(&self) -> Result<(), RepositoryError> {
        self.state.lock().unwrap().0 = false;
        Ok(())
    }

    async fn acquire_lease(
        &self,
        owner: uuid::Uuid,
        _now: DateTime<Utc>,
        _ttl_seconds: i64,
    ) -> Result<bool, RepositoryError> {
        let mut state = self.state.lock().unwrap();
        if state.1.is_some() {
            return Ok(false);
        }
        state.1 = Some(owner);
        Ok(true)
    }

    async fn reserve_email(&self, owner: uuid::Uuid, _cap: u32) -> Result<bool, RepositoryError> {
        Ok(self.state.lock().unwrap().1 == Some(owner))
    }

    async fn release_lease(&self, owner: uuid::Uuid) -> Result<(), RepositoryError> {
        let mut state = self.state.lock().unwrap();
        if state.1 == Some(owner) {
            state.1 = None;
        }
        Ok(())
    }
}

struct FakeContacts;

#[async_trait]
impl ContactDirectory for FakeContacts {
    async fn verified_contact(&self, _user_id: &str) -> Result<Option<String>, ExternalError> {
        Ok(Some("user@example.com".to_string()))
    }
}

struct FakeEmail;

#[async_trait]
impl EmailSender for FakeEmail {
    async fn send(&self, _message: &EmailMessage) -> Result<(), ExternalError> {
        Ok(())
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
            Err("postgres is down".to_string())
        }
    }
}

fn caller(service: &str, operations: &[&str]) -> ServiceCaller {
    ServiceCaller {
        service: service.to_string(),
        operations: operations
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    }
}

struct Harness {
    state: AppState,
    clock: Arc<MutableClock>,
    store: Arc<Store>,
}

fn harness() -> Harness {
    harness_with(true, false)
}

fn harness_with(ready: bool, lease_held: bool) -> Harness {
    let store = Arc::new(Store::default());
    let alerts = Arc::new(FakeAlerts(store.clone()));
    let attempts = Arc::new(FakeAttempts(store.clone()));
    let preferences = Arc::new(FakePreferences(store.clone()));
    let control = Arc::new(FakeControl::default());
    if lease_held {
        control.state.lock().unwrap().1 = Some(uuid::Uuid::new_v4());
    }
    let clock = Arc::new(MutableClock::at(at(9)));
    let ids = Arc::new(SequentialIds::default());
    let audit = Arc::new(FakeAudit::default());
    let trail = Arc::new(Trail::new(audit, clock.clone(), Arc::new(NoPause)));

    let submit = Arc::new(SubmitAlert::new(
        alerts.clone(),
        control.clone(),
        ids.clone(),
        clock.clone(),
    ));
    let preference_service = Arc::new(PreferenceService::new(
        preferences.clone(),
        ids.clone(),
        clock.clone(),
        trail.clone(),
    ));
    let feed = Arc::new(FeedService::new(alerts.clone()));
    let delivery = Arc::new(DeliveryService::new(
        alerts.clone(),
        preferences.clone(),
        attempts.clone(),
        control.clone(),
        Arc::new(FakeContacts),
        Arc::new(FakeEmail),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let retention = Arc::new(RetentionService::new(
        alerts.clone(),
        attempts.clone(),
        clock.clone(),
        Arc::new(NoPause),
    ));
    let erase = Arc::new(EraseService::new(alerts, ids, trail.clone()));

    let token_validator = Arc::new(FakeTokens {
        entries: vec![
            (
                "user-token".to_string(),
                Ok(Principal {
                    subject: USER.to_string(),
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
        ],
    });
    let service_tokens = Arc::new(FakeServiceTokens {
        entries: vec![
            (
                "account-token".to_string(),
                caller("account", &["alerts.submit"]),
            ),
            (
                "portfolio-token".to_string(),
                caller("portfolio", &["alerts.submit"]),
            ),
            (
                "identity-token".to_string(),
                caller("identity", &["privacy.anonymise"]),
            ),
            (
                "smoke-token".to_string(),
                caller("smoke-reader", &["smoke"]),
            ),
            (
                "run-token".to_string(),
                caller("job-orchestration", &["runs"]),
            ),
        ],
    });

    let state = AppState {
        service_name: "notification-api",
        submit,
        preferences: preference_service,
        feed,
        delivery,
        retention,
        erase,
        trail,
        service_tokens,
        token_validator,
        readiness: Arc::new(Ready(ready)),
        clock: clock.clone(),
        metrics: Arc::new(notification_http::metrics::Metrics::default()),
    };

    Harness {
        state,
        clock,
        store,
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

fn alert_body(kind: &str, key: &str) -> serde_json::Value {
    serde_json::json!({
        "userId": USER,
        "type": kind,
        "idempotencyKey": key,
        "payload": {"assetId": "bitcoin", "driftPoints": "7.5"}
    })
}

// ---------------------------------------------------------------------------
// Probes and metrics
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_is_live() {
    let harness = harness();
    let (status, body) = call(api_router(harness.state), get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "notification-api");
}

#[tokio::test]
async fn readiness_reports_ready() {
    let harness = harness();
    let (status, body) = call(api_router(harness.state), get("/readyz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn readiness_returns_503_when_postgres_is_down() {
    let harness = harness_with(false, false);
    let (status, body) = call(api_router(harness.state), get("/readyz")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], 503);
}

#[tokio::test]
async fn metrics_expose_the_required_names() {
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
    for name in [
        "notification_health",
        "notification_request_errors_total",
        "notification_request_duration_seconds",
        "notification_alerts_total",
        "notification_attempts_total",
        "notification_duplicates_total",
        "notification_mandatory_delivery_total",
        "notification_audit_failures_total",
    ] {
        assert!(text.contains(name), "missing metric {name}");
    }
}

// ---------------------------------------------------------------------------
// C9 submit
// ---------------------------------------------------------------------------

#[tokio::test]
async fn submit_accepts_a_service_call() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], "queued");
    assert_eq!(harness.store.alerts.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn submit_reports_deduplicated() {
    let harness = harness();
    let (first_status, first) = call(
        api_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    assert_eq!(first_status, StatusCode::ACCEPTED);
    assert_eq!(first["status"], "queued");

    let (second_status, second) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    assert_eq!(second_status, StatusCode::ACCEPTED);
    assert_eq!(second["status"], "deduplicated");
    assert_eq!(second["alertId"], first["alertId"]);
    assert_eq!(harness.store.alerts.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn submit_requires_a_service_token() {
    let harness = harness();
    let request = Request::builder()
        .method("POST")
        .uri("/v1/alerts")
        .header("content-type", "application/json")
        .body(Body::from(alert_body("drift", "drift:1").to_string()))
        .unwrap();
    let (status, _) = call(api_router(harness.state), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn submit_refuses_a_token_without_the_operation() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/alerts",
            "identity-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn submit_rejects_a_blank_key() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(serde_json::json!({
                "userId": USER,
                "type": "drift",
                "idempotencyKey": "  "
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "idempotencyKey");
}

#[tokio::test]
async fn submit_rejects_a_bad_body() {
    let harness = harness();
    let request = Request::builder()
        .method("POST")
        .uri("/v1/alerts")
        .header("authorization", "Bearer account-token")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let (status, body) = call(api_router(harness.state), request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "body");
}

#[tokio::test]
async fn metrics_count_a_duplicate() {
    let harness = harness();
    let router = api_router(harness.state.clone());
    for _ in 0..2 {
        let _ = call(
            router.clone(),
            service_request(
                "POST",
                "/v1/alerts",
                "account-token",
                Some(alert_body("drift", "drift:1")),
            ),
        )
        .await;
    }
    let response = router.oneshot(get("/metrics")).await.unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("notification_duplicates_total"));
}

// ---------------------------------------------------------------------------
// C1 feed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_feed_returns_the_callers_alerts() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications"),
            "user-token",
            USER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["alerts"][0]["type"], "drift");
    assert_eq!(body["alerts"][0]["payload"]["assetId"], "bitcoin");
    assert_eq!(body["alerts"][0]["status"], "queued");
}

#[tokio::test]
async fn the_feed_refuses_another_users_path() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications"),
            "other-token",
            OTHER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_feed_requires_a_jwt() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        get(&format!("/v1/users/{USER}/notifications")),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_feed_rejects_a_mismatched_subject_header() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications"),
            "user-token",
            OTHER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_feed_returns_a_cursor() {
    let harness = harness();
    for index in 0..2 {
        harness.clock.advance(ChronoDuration::minutes(1));
        let _ = call(
            api_router(harness.state.clone()),
            service_request(
                "POST",
                "/v1/alerts",
                "account-token",
                Some(alert_body("drift", &format!("drift:{index}"))),
            ),
        )
        .await;
    }
    let (status, body) = call(
        api_router(harness.state.clone()),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications?limit=1"),
            "user-token",
            USER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["alerts"].as_array().unwrap().len(), 1);
    let cursor = body["nextCursor"].as_str().expect("a cursor").to_string();

    let (status, body) = call(
        api_router(harness.state.clone()),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications?limit=1&cursor={cursor}"),
            "user-token",
            USER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["alerts"].as_array().unwrap().len(), 1);
    let cursor = body["nextCursor"]
        .as_str()
        .expect("a full page has a cursor")
        .to_string();

    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications?limit=1&cursor={cursor}"),
            "user-token",
            USER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["alerts"].as_array().unwrap().is_empty());
    assert!(body["nextCursor"].is_null());
}

#[tokio::test]
async fn the_feed_hides_a_type_disabled_for_in_app() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let (status, _) = call(
        api_router(harness.state.clone()),
        user_request(
            "PUT",
            &format!("/v1/users/{USER}/notifications/preferences"),
            "user-token",
            USER,
            Some(serde_json::json!({
                "preferences": [
                    {"type": "drift", "inApp": false, "email": false}
                ]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications"),
            "user-token",
            USER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["alerts"].as_array().unwrap().is_empty(),
        "BR2.1: a type disabled for in-app never reaches the feed"
    );
}

#[tokio::test]
async fn request_metrics_are_labelled_by_route_and_status() {
    let harness = harness();
    let router = api_router(harness.state);
    let (status, _) = call(router.clone(), get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    let response = router.oneshot(get("/metrics")).await.unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("route=\"/healthz\""));
    assert!(text.contains("status=\"200\""));
}

#[tokio::test]
async fn a_poll_driven_run_records_delivery_metrics() {
    // The worker's five-second poll runs `poll` and then meters the report with
    // this same shared function (services/notification/src/lib.rs).
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(serde_json::json!({
                "userId": USER,
                "type": "security",
                "idempotencyKey": "security:poll",
                "payload": {"event": "new device"}
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let report = harness
        .state
        .delivery
        .poll()
        .await
        .expect("the poll succeeds")
        .expect("a due alert starts a run");
    notification_http::metrics::record_delivery_report(&harness.state.metrics, &report);

    let response = api_router(harness.state)
        .oneshot(get("/metrics"))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        text.contains(
            "notification_alerts_total{service=\"notification-api\",type=\"security\",status=\"delivered\"}"
        ),
        "the poll path records alert outcomes"
    );
    assert!(
        text.contains("notification_attempts_total{service=\"notification-api\",channel=\"email\""),
        "the poll path records attempts"
    );
    assert!(
        text.contains(
            "notification_mandatory_delivery_total{service=\"notification-api\",type=\"security\",status=\"delivered\"}"
        ),
        "the poll path records the mandatory outcome"
    );
}

// ---------------------------------------------------------------------------
// C1 preferences
// ---------------------------------------------------------------------------

#[tokio::test]
async fn preferences_default_for_a_new_user() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications/preferences"),
            "user-token",
            USER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let preferences = body["preferences"].as_array().unwrap();
    assert_eq!(preferences.len(), 5);
    let drift = preferences
        .iter()
        .find(|entry| entry["type"] == "drift")
        .unwrap();
    assert_eq!(drift["inApp"], true);
    assert_eq!(drift["email"], false);
    let brief = preferences
        .iter()
        .find(|entry| entry["type"] == "brief")
        .unwrap();
    assert_eq!(brief["email"], true);
}

#[tokio::test]
async fn saving_preferences_applies() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "PUT",
            &format!("/v1/users/{USER}/notifications/preferences"),
            "user-token",
            USER,
            Some(serde_json::json!({
                "preferences": [
                    {"type": "drift", "inApp": true, "email": true}
                ]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let drift = body["preferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["type"] == "drift")
        .unwrap()
        .clone();
    assert_eq!(drift["email"], true);
    assert_eq!(harness.store.preferences.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn saving_a_mandatory_email_off_is_refused() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        user_request(
            "PUT",
            &format!("/v1/users/{USER}/notifications/preferences"),
            "user-token",
            USER,
            Some(serde_json::json!({
                "preferences": [
                    {"type": "security", "inApp": true, "email": false}
                ]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "security.email");
}

#[tokio::test]
async fn preferences_refuse_another_users_path() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        user_request(
            "GET",
            &format!("/v1/users/{USER}/notifications/preferences"),
            "other-token",
            OTHER,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// C9 erasure
// ---------------------------------------------------------------------------

#[tokio::test]
async fn erase_returns_204_for_identity() {
    let harness = harness();
    let _ = call(
        api_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    let (status, _) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/privacy/anonymise",
            "identity-token",
            Some(serde_json::json!({"userId": USER, "idempotencyKey": "erase-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_ne!(
        harness.store.alerts.lock().unwrap()[0]
            .user_id()
            .as_uuid()
            .to_string(),
        USER
    );
}

#[tokio::test]
async fn erase_refuses_another_service() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/privacy/anonymise",
            "account-token",
            Some(serde_json::json!({"userId": USER, "idempotencyKey": "erase-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn erase_rejects_a_blank_idempotency_key() {
    let harness = harness();
    let (status, body) = call(
        api_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/privacy/anonymise",
            "identity-token",
            Some(serde_json::json!({"userId": USER, "idempotencyKey": " "})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "idempotencyKey");
}

// ---------------------------------------------------------------------------
// C11 runs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn run_delivery_returns_a_result() {
    let harness = harness();
    let _ = call(
        api_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(alert_body("drift", "drift:1")),
        ),
    )
    .await;
    let (status, body) = call(
        worker_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/run/notification-deliver",
            "run-token",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jobName"], "notification-deliver");
    assert_eq!(body["status"], "succeeded");
    assert!(body["runId"].is_string());
}

#[tokio::test]
async fn a_second_run_while_leased_is_409() {
    let harness = harness_with(true, true);
    let (status, body) = call(
        worker_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/run/notification-deliver",
            "run-token",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["status"], 409);
}

#[tokio::test]
async fn run_retention_returns_counts() {
    let harness = harness();
    let (status, body) = call(
        worker_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/run/notification-retention",
            "run-token",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jobName"], "notification-retention");
    assert_eq!(body["detail"], "0 alerts and 0 attempts deleted");
}

#[tokio::test]
async fn the_run_path_refuses_a_user_token() {
    let harness = harness();
    let (status, _) = call(
        worker_router(harness.state),
        service_request(
            "POST",
            "/v1/internal/run/notification-deliver",
            "user-token",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Problems and correlation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unknown_path_returns_a_problem() {
    let harness = harness();
    let (status, body) = call(api_router(harness.state), get("/v1/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert!(body["correlationId"].is_string());
}

#[tokio::test]
async fn the_correlation_id_is_echoed() {
    let harness = harness();
    let response = api_router(harness.state)
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .header("x-correlation-id", "corr-42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .headers()
            .get("x-correlation-id")
            .and_then(|value| value.to_str().ok()),
        Some("corr-42")
    );
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state),
        Request::builder()
            .uri("/healthz")
            .header("x-correlation-id", "bad id with spaces")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_worker_serves_its_probes() {
    let harness = harness();
    let (status, body) = call(worker_router(harness.state), get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["service"], "notification-api");
}

#[tokio::test]
async fn a_payload_is_sanitised_before_it_is_stored() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state.clone()),
        service_request(
            "POST",
            "/v1/alerts",
            "account-token",
            Some(serde_json::json!({
                "userId": USER,
                "type": "security",
                "idempotencyKey": "security:1",
                "payload": {"event": "alice@example.com", "deviceLabel": "Chrome"}
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let alert = harness.store.alerts.lock().unwrap()[0].clone();
    assert!(!alert.payload().contains_key("event"));
    assert_eq!(alert.sanitised_fields(), &["event".to_string()]);
    assert_eq!(
        alert.kind(),
        notification_domain::NotificationType::Security
    );
}

#[tokio::test]
async fn the_preference_refusal_is_audited() {
    let harness = harness();
    let (status, _) = call(
        api_router(harness.state.clone()),
        user_request(
            "PUT",
            &format!("/v1/users/{USER}/notifications/preferences"),
            "user-token",
            USER,
            Some(serde_json::json!({
                "preferences": [
                    {"type": "key_revoked", "inApp": true, "email": false}
                ]
            })),
        ),
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
    assert!(text.contains("notification_audit_failures_total"));
}

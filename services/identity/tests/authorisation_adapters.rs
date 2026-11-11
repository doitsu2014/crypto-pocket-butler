//! Adapter tests for the two security-critical authorisation adapters, against
//! a stub HTTP server: the audit field set and redaction, and the provider role
//! write that must preserve the rest of `app_metadata`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use chrono::{TimeZone, Utc};
use identity_application::authorisation::ports::{AuditSink, AuthorisationEvent, RoleWriter};
use identity_application::ports::{ExternalError, RepositoryError};
use identity_domain::{AuditOutcome, DenialReason, Role};
use identity_infrastructure::authorisation::audit::AuthorisationAuditSink;
use identity_infrastructure::authorisation::role_writer::SupabaseRoleWriter;
use serde_json::{json, Value};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
struct Recorded {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: String,
}

struct Stub {
    get_status: AtomicU16,
    put_status: AtomicU16,
    post_status: AtomicU16,
    metadata: Mutex<String>,
    requests: Mutex<Vec<Recorded>>,
}

impl Default for Stub {
    fn default() -> Self {
        Self {
            get_status: AtomicU16::new(200),
            put_status: AtomicU16::new(200),
            post_status: AtomicU16::new(202),
            metadata: Mutex::new("{}".to_string()),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl Stub {
    fn record(&self, method: &str, path: &str, headers: &HeaderMap, body: &str) {
        self.requests.lock().unwrap().push(Recorded {
            method: method.to_string(),
            path: path.to_string(),
            headers: headers
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (name.as_str().to_ascii_lowercase(), value.to_string()))
                })
                .collect(),
            body: body.to_string(),
        });
    }

    fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

async fn get_user(
    State(stub): State<Arc<Stub>>,
    axum::extract::Path(subject): axum::extract::Path<String>,
    headers: HeaderMap,
) -> (StatusCode, String) {
    stub.record("GET", &format!("/admin/users/{subject}"), &headers, "");
    let status = StatusCode::from_u16(stub.get_status.load(Ordering::Relaxed))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, stub.metadata.lock().unwrap().clone())
}

async fn put_user(
    State(stub): State<Arc<Stub>>,
    axum::extract::Path(subject): axum::extract::Path<String>,
    headers: HeaderMap,
    body: String,
) -> StatusCode {
    stub.record("PUT", &format!("/admin/users/{subject}"), &headers, &body);
    StatusCode::from_u16(stub.put_status.load(Ordering::Relaxed))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn events(State(stub): State<Arc<Stub>>, headers: HeaderMap, body: String) -> StatusCode {
    stub.record("POST", "/v1/events", &headers, &body);
    StatusCode::from_u16(stub.post_status.load(Ordering::Relaxed))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn router(stub: Arc<Stub>) -> Router {
    Router::new()
        .route("/admin/users/{subject}", get(get_user).put(put_user))
        .route("/v1/events", post(events))
        .with_state(stub)
}

async fn serve(stub: Arc<Stub>) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router(stub);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{address}")
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 10, 9, 0, 0).unwrap()
}

#[tokio::test]
async fn the_audit_sink_posts_the_fixed_field_set_and_the_correlation_id() {
    let stub = Arc::new(Stub::default());
    let sink = AuthorisationAuditSink::new(serve(stub.clone()).await, "svc-token");
    let event = AuthorisationEvent::denied(
        "user-1",
        "principal:read",
        "account:user-1",
        now(),
        DenialReason::NotOwner,
    );

    sink.record(&event, "corr-1").await.unwrap();

    let requests = stub.requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/events");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer svc-token")
    );
    assert!(
        request
            .headers
            .get("idempotency-key")
            .is_some_and(|key| !key.is_empty()),
        "the write carries an idempotency key"
    );

    let body: Value = serde_json::from_str(&request.body).expect("the audit body is JSON");
    assert_eq!(body["actor"], "user-1");
    assert_eq!(body["action"], "principal:read");
    assert_eq!(body["target"], "account:user-1");
    assert_eq!(body["outcome"], "denied");
    assert_eq!(body["context"]["reason"], "not_owner");
    assert_eq!(body["context"]["correlationId"], "corr-1");
}

#[tokio::test]
async fn the_audit_sink_surfaces_a_4xx_and_a_5xx() {
    let stub = Arc::new(Stub::default());
    let sink = AuthorisationAuditSink::new(serve(stub.clone()).await, "svc-token");
    let event = AuthorisationEvent::denied(
        "user-1",
        "principal:read",
        "account:user-1",
        now(),
        DenialReason::NoRole,
    );

    stub.post_status.store(400, Ordering::Relaxed);
    assert!(matches!(
        sink.record(&event, "corr").await,
        Err(RepositoryError::Unavailable(_))
    ));

    stub.post_status.store(500, Ordering::Relaxed);
    assert!(matches!(
        sink.record(&event, "corr").await,
        Err(RepositoryError::Unavailable(_))
    ));
}

#[tokio::test]
async fn the_audit_sink_refuses_a_token_shaped_value_without_sending_it() {
    let stub = Arc::new(Stub::default());
    let sink = AuthorisationAuditSink::new(serve(stub.clone()).await, "svc-token");
    let event = AuthorisationEvent::denied(
        "user-1",
        "principal:read",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.signature",
        now(),
        DenialReason::NotOwner,
    );

    assert!(matches!(
        sink.record(&event, "corr").await,
        Err(RepositoryError::Validation(_))
    ));
    assert!(
        stub.requests().is_empty(),
        "a refused write never reaches the audit service"
    );
}

#[tokio::test]
async fn the_audit_sink_refuses_a_wallet_shaped_value_without_sending_it() {
    let stub = Arc::new(Stub::default());
    let sink = AuthorisationAuditSink::new(serve(stub.clone()).await, "svc-token");
    let wallet = format!("0x{}", "a".repeat(40));
    let event = AuthorisationEvent::role_change_requested(
        "admin-1",
        "user-2",
        now(),
        Some(Role::User),
        Role::Admin,
        &wallet,
        AuditOutcome::Success,
    );

    assert!(matches!(
        sink.record(&event, "corr").await,
        Err(RepositoryError::Validation(_))
    ));
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn the_role_writer_merges_the_role_and_keeps_the_rest_of_app_metadata() {
    let stub = Arc::new(Stub::default());
    *stub.metadata.lock().unwrap() = json!({
        "app_metadata": {
            "has_exchange_credentials": true,
            "display_name": "Ada"
        }
    })
    .to_string();
    let writer = SupabaseRoleWriter::new(serve(stub.clone()).await, "service-role-key");

    writer.set_role("subject-1", Role::Admin).await.unwrap();

    let requests = stub.requests();
    assert_eq!(requests.len(), 2, "a read then a merged write");
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[1].method, "PUT");
    assert_eq!(
        requests[1].headers.get("authorization").map(String::as_str),
        Some("Bearer service-role-key")
    );
    let body: Value = serde_json::from_str(&requests[1].body).unwrap();
    assert_eq!(body["app_metadata"]["role"], "admin");
    assert_eq!(body["app_metadata"]["has_exchange_credentials"], true);
    assert_eq!(body["app_metadata"]["display_name"], "Ada");
}

#[tokio::test]
async fn the_role_writer_surfaces_a_refused_and_an_unavailable_provider() {
    let stub = Arc::new(Stub::default());
    let writer = SupabaseRoleWriter::new(serve(stub.clone()).await, "service-role-key");

    // The provider refuses the update: no retry, a surfaced refusal.
    stub.put_status.store(403, Ordering::Relaxed);
    assert!(matches!(
        writer.set_role("subject-1", Role::Admin).await,
        Err(ExternalError::Refused(_))
    ));

    // The metadata read fails: the write is never attempted.
    stub.get_status.store(500, Ordering::Relaxed);
    let before = stub.requests().len();
    assert!(matches!(
        writer.set_role("subject-2", Role::Admin).await,
        Err(ExternalError::Unavailable(_))
    ));
    assert_eq!(
        stub.requests().len(),
        before + 1,
        "a failed metadata read never reaches the write"
    );

    // The provider answers 5xx on the write.
    stub.metadata
        .lock()
        .unwrap()
        .clone_from(&json!({ "app_metadata": {} }).to_string());
    stub.get_status.store(200, Ordering::Relaxed);
    stub.put_status.store(503, Ordering::Relaxed);
    assert!(matches!(
        writer.set_role("subject-3", Role::Admin).await,
        Err(ExternalError::Unavailable(_))
    ));
}

//! The composition root: router assembly, the probes it serves, the worker
//! heartbeat, and — against a disposable Postgres — `build_api_state` and
//! `build_worker_runtime`.
//!
//! These are the parts of `services/identity/src/` that can be exercised
//! without binding a port or receiving a signal. The bodies of `run` and
//! `run_worker` after the state is built, and the supervisor's signal wait,
//! still need a real listener and a shutdown, so the crate keeps a documented
//! remainder after this suite.
//!
//! The adapter-backed cases need `TEST_DATABASE_URL` and are `#[ignore]`d, like
//! the rest of the integration suite.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use identity::{router, write_heartbeat, ApiConfig, WorkerConfig};
use identity_http::AppState;
use tower::ServiceExt;

/// The token registry the composition root expects to present to other services.
const TOKEN_JSON: &str = r#"{"services":{"identity":"test-token"}}"#;

/// A scratch directory inside the workspace, removed by the test.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!(
            "identity-composition-test-{name}-{}",
            std::process::id()
        ));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
}

async fn call(state: AppState, path: &str) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("build the probe request"),
        )
        .await
        .expect("the router answers")
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("read the response body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("the body is UTF-8")
}

fn api_config(token_path: &Path, database_url: &str) -> ApiConfig {
    ApiConfig {
        port: 0,
        database_url: database_url.to_string(),
        service_tokens_path: token_path.to_path_buf(),
        jwks_url: "http://127.0.0.1:9/jwks.json".to_string(),
        jwt_issuer: "https://auth.test".to_string(),
        jwt_audience: "authenticated".to_string(),
        supabase_auth_url: "http://127.0.0.1:9".to_string(),
        service_role_key: "service-role".to_string(),
        storage_url: "http://127.0.0.1:9".to_string(),
        export_bucket: "identity-exports".to_string(),
        audit_url: "http://127.0.0.1:9".to_string(),
        account_url: "http://127.0.0.1:9".to_string(),
        ledger_url: "http://127.0.0.1:9".to_string(),
        portfolio_url: "http://127.0.0.1:9".to_string(),
        worker_url: "http://127.0.0.1:9".to_string(),
        token_service: "identity".to_string(),
        gate_token_key: "gate-key".to_string(),
        jurisdiction_provider_url: String::new(),
        jurisdiction_provider_key: None,
        authorize_services: vec!["account-api".to_string()],
        consent_services: vec!["account-api".to_string()],
        drainer_interval_seconds: 5,
        pool_connections: 2,
    }
}

fn worker_config(token_path: &Path, database_url: &str) -> WorkerConfig {
    WorkerConfig {
        port: 0,
        database_url: database_url.to_string(),
        retention_database_url: database_url.to_string(),
        service_tokens_path: token_path.to_path_buf(),
        supabase_auth_url: "http://127.0.0.1:9".to_string(),
        service_role_key: "service-role".to_string(),
        storage_url: "http://127.0.0.1:9".to_string(),
        export_bucket: "identity-exports".to_string(),
        audit_url: "http://127.0.0.1:9".to_string(),
        account_url: "http://127.0.0.1:9".to_string(),
        ledger_url: "http://127.0.0.1:9".to_string(),
        portfolio_url: "http://127.0.0.1:9".to_string(),
        token_service: "identity".to_string(),
        sweep_interval_seconds: 60,
        pool_connections: 2,
    }
}

/// Creates the `identity` schema the pooled connection pins its search path to.
async fn prepare_identity_schema(database_url: &str) {
    let admin = sqlx::PgPool::connect(database_url)
        .await
        .expect("connect to the disposable database");
    sqlx::query("CREATE SCHEMA IF NOT EXISTS identity")
        .execute(&admin)
        .await
        .expect("create the identity schema");
    sqlx::query("DROP TABLE IF EXISTS identity._sqlx_migrations")
        .execute(&admin)
        .await
        .expect("clear any stale migration table");
    admin.close().await;
}

#[tokio::test]
async fn the_router_answers_its_probes_with_the_security_headers() {
    let harness = common::harness();

    let health = call(harness.state.clone(), "/healthz").await;
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(
        health
            .headers()
            .get(header::X_CONTENT_TYPE_OPTIONS)
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        health
            .headers()
            .get(header::STRICT_TRANSPORT_SECURITY)
            .and_then(|value| value.to_str().ok()),
        Some("max-age=63072000; includeSubDomains")
    );
    let text = body_text(health).await;
    assert!(text.contains("\"ok\""), "{text}");
    assert!(text.contains("\"identity-api\""), "{text}");

    let ready = call(harness.state.clone(), "/readyz").await;
    assert_eq!(ready.status(), StatusCode::OK);
    let text = body_text(ready).await;
    assert!(text.contains("\"ready\""), "{text}");
}

#[tokio::test]
async fn a_failed_readiness_probe_is_a_problem_response() {
    let mut state = common::harness().state;
    state.readiness = Arc::new(common::fakes::Ready(false));

    let response = call(state, "/readyz").await;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );
    assert!(
        response.headers().get("x-correlation-id").is_some(),
        "every response carries the correlation id"
    );
    let text = body_text(response).await;
    assert!(text.contains("not-ready"), "{text}");
}

#[tokio::test]
async fn an_unknown_route_answers_the_fallback_and_is_counted() {
    let harness = common::harness();

    let response = call(harness.state.clone(), "/v1/no-such-route").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );
    let text = body_text(response).await;
    assert!(text.contains("the route does not exist"), "{text}");

    let metrics = call(harness.state, "/metrics").await;
    assert_eq!(metrics.status(), StatusCode::OK);
    let text = body_text(metrics).await;
    assert!(
        text.contains(
            "http_requests_total{service=\"identity-api\",status_class=\"4xx\",route=\"other\"} 1"
        ),
        "{text}"
    );
}

#[test]
fn the_heartbeat_records_a_timestamp_and_a_failed_write_is_logged() {
    let dir = scratch("heartbeat");
    let path = dir.join("heartbeat");

    write_heartbeat(&path);

    let text = std::fs::read_to_string(&path).expect("the heartbeat file exists");
    assert!(
        chrono::DateTime::parse_from_rfc3339(text.trim()).is_ok(),
        "the heartbeat is an RFC 3339 instant: {text}"
    );

    // A directory cannot be written: the failure is warned about, never panicked
    // and never fatal to the supervisor.
    write_heartbeat(&dir);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn build_api_state_assembles_the_api_and_its_probes() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at a disposable Postgres database");
    prepare_identity_schema(&database_url).await;
    let dir = scratch("build-api-state");
    let token_path = dir.join("service-tokens.json");
    std::fs::write(&token_path, TOKEN_JSON).expect("write the token registry");

    let runtime = identity::build_api_state(&api_config(&token_path, &database_url))
        .await
        .expect("the composition root builds from an un-migrated schema");

    let health = call(runtime.state.clone(), "/healthz").await;
    assert_eq!(health.status(), StatusCode::OK);

    let ready = call(runtime.state.clone(), "/readyz").await;
    assert_eq!(
        ready.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "a schema without the migration table is not ready"
    );
    let text = body_text(ready).await;
    assert!(text.contains("not-ready"), "{text}");

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn build_api_state_names_a_registry_without_the_outbound_token() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at a disposable Postgres database");
    prepare_identity_schema(&database_url).await;
    let dir = scratch("missing-token");
    let token_path = dir.join("service-tokens.json");
    std::fs::write(&token_path, r#"{"services":{"account-api":"test-token"}}"#)
        .expect("write the token registry");

    let error = match identity::build_api_state(&api_config(&token_path, &database_url)).await {
        Ok(_) => panic!("a registry without the outbound token must be refused"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("has no token for 'identity'"), "{error}");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn build_api_state_refuses_an_unreadable_token_registry() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at a disposable Postgres database");
    prepare_identity_schema(&database_url).await;
    let missing = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("identity-composition-test-absent-service-tokens.json");

    let error = match identity::build_api_state(&api_config(&missing, &database_url)).await {
        Ok(_) => panic!("an unreadable token registry must be refused"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("is unreadable"), "{error}");
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn build_worker_runtime_wires_the_worker_without_binding_a_port() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at a disposable Postgres database");
    prepare_identity_schema(&database_url).await;
    let dir = scratch("build-worker-runtime");
    let token_path = dir.join("service-tokens.json");
    std::fs::write(&token_path, TOKEN_JSON).expect("write the token registry");

    let runtime = identity::build_worker_runtime(&worker_config(&token_path, &database_url))
        .await
        .expect("the worker composition root builds from an un-migrated schema");

    assert_eq!(
        runtime.port, 0,
        "the configured port is carried to the bind"
    );
    assert_eq!(runtime.state.service_name, "identity-worker");
    assert!(
        runtime.state.readiness.check().await.is_err(),
        "an un-migrated schema is not ready"
    );

    std::fs::remove_dir_all(&dir).ok();
}

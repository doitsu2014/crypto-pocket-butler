//! Gateway infrastructure adapters against a real socket: the JWKS fetch, the
//! readiness probe, the clock, and the audit sink.
//!
//! Each test runs a one-shot HTTP responder on a loopback port and asserts what
//! the adapter reports: the parsed keys, the refusal reason, or the readiness
//! outcome. These are the behaviours the gateway's own readiness and 503 paths
//! depend on.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use chrono::Utc;
use gateway_application::ports::{AuditEvent, AuditSink, Clock, ReadinessProbe};
use gateway_infrastructure::auth::JwksSource;
use gateway_infrastructure::{HttpJwksSource, HttpReadinessProbe, SystemClock, TracingAuditSink};

/// Serves exactly one HTTP request with a fixed status and body, then stops.
fn serve_once(status: u16, body: &str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let body = body.to_string();
    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (format!("http://{addr}/jwks"), handle)
}

const JWKS: &str = r#"{"keys":[
  {"kid":"key-one","kty":"RSA","n":"AQAB","e":"AQAB"},
  {"kid":"oct-key","kty":"oct","k":"c2VjcmV0"},
  {"kty":"RSA","n":"AQAB","e":"AQAB"},
  {"kid":"no-exponent","kty":"RSA","n":"AQAB"}
]}"#;

#[tokio::test]
async fn the_jwks_source_keeps_only_usable_rsa_keys() {
    let (url, server) = serve_once(200, JWKS);

    let keys = HttpJwksSource::new(url).fetch().await.unwrap();

    server.join().unwrap();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].kid, "key-one");
    assert_eq!(keys[0].n, "AQAB");
    assert_eq!(keys[0].e, "AQAB");
    // A key with no `kid` is still usable and is named after its position.
    assert_eq!(keys[1].kid, "key-2");
}

#[tokio::test]
async fn the_jwks_source_refuses_a_document_with_no_rsa_keys() {
    let (url, server) = serve_once(
        200,
        r#"{"keys":[{"kid":"oct","kty":"oct","k":"c2VjcmV0"}]}"#,
    );

    let error = HttpJwksSource::new(url).fetch().await.unwrap_err();

    server.join().unwrap();
    assert_eq!(error, "JWKS document contained no RSA signing keys");
}

#[tokio::test]
async fn the_jwks_source_refuses_an_error_response() {
    let (url, server) = serve_once(500, r#"{"error":"upstream is down"}"#);

    let error = HttpJwksSource::new(url).fetch().await.unwrap_err();

    server.join().unwrap();
    assert!(error.contains("500"), "{error}");
}

#[tokio::test]
async fn the_jwks_source_reports_an_unreachable_endpoint() {
    // Port 1 is reserved and never accepts a connection.
    let error = HttpJwksSource::new("http://127.0.0.1:1/jwks")
        .fetch()
        .await
        .unwrap_err();

    assert!(!error.is_empty());
}

#[tokio::test]
async fn the_readiness_probe_reports_ready_for_a_2xx_endpoint() {
    let (url, server) = serve_once(200, r#"{"status":"ready"}"#);

    let result = HttpReadinessProbe::new(url).check().await;

    server.join().unwrap();
    assert!(result.is_ok());
}

#[tokio::test]
async fn the_readiness_probe_reports_not_ready_for_a_5xx_endpoint() {
    let (url, server) = serve_once(503, r#"{"status":"not ready"}"#);

    let error = HttpReadinessProbe::new(url).check().await.unwrap_err();

    server.join().unwrap();
    assert!(error.contains("503"), "{error}");
}

#[tokio::test]
async fn the_readiness_probe_reports_an_unreachable_dependency() {
    let error = HttpReadinessProbe::new("http://127.0.0.1:1/readyz")
        .check()
        .await
        .unwrap_err();

    assert!(!error.is_empty());
}

#[tokio::test]
async fn the_system_clock_reports_the_current_instant() {
    let before = Utc::now();
    let now = SystemClock.now();
    let after = Utc::now();

    assert!(now >= before && now <= after);
}

#[tokio::test]
async fn the_audit_sink_accepts_a_complete_event() {
    let event = AuditEvent::new(
        "anonymous",
        "request.denied",
        "/v1/assets",
        "denied",
        Some("corr-1".to_string()),
        Utc::now(),
    )
    .unwrap();

    let result = TracingAuditSink.record(&event).await;

    assert!(result.is_ok());
}

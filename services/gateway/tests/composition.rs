//! The gateway's composition root: configuration from the environment, the
//! state it assembles, and the listener it runs.
//!
//! These tests assert what an operator observes — the value each variable
//! produces, the named refusal for a missing or malformed one, and the fact that
//! the bound port serves the gateway's own endpoints.

use std::io::{Read, Write};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gateway::Config;
use gateway_http::router;
use http_body_util::BodyExt;
use tower::ServiceExt;

#[path = "common/mod.rs"]
mod common;

use common::jwt::{sign, test_key, TestClaims};

const ENV_VARS: [&str; 9] = [
    "GATEWAY_PORT",
    "ASSET_REGISTRY_URL",
    "IDENTITY_URL",
    "JWT_JWKS_URL",
    "JWT_ISSUER",
    "JWT_AUDIENCE",
    "UPSTREAM_TIMEOUT_MS",
    "GATEWAY_RATE_LIMIT_PER_MINUTE",
    "GATEWAY_ANONYMOUS_RATE_LIMIT_PER_MINUTE",
];

fn clear_env() {
    for name in ENV_VARS {
        std::env::remove_var(name);
    }
}

/// One test owns the process environment: every case runs in sequence so no
/// other test observes a variable it did not set.
#[test]
fn configuration_reads_every_variable_names_each_refusal_and_keeps_its_defaults() {
    clear_env();

    // Every variable set: each value reaches the struct unchanged.
    std::env::set_var("GATEWAY_PORT", "9090");
    std::env::set_var("ASSET_REGISTRY_URL", "http://registry.internal:8081");
    std::env::set_var("IDENTITY_URL", "http://identity.internal:8083");
    std::env::set_var(
        "JWT_JWKS_URL",
        "https://project.supabase.co/auth/v1/.well-known/jwks.json",
    );
    std::env::set_var("JWT_ISSUER", "https://project.supabase.co/auth/v1");
    std::env::set_var("JWT_AUDIENCE", "authenticated");
    std::env::set_var("UPSTREAM_TIMEOUT_MS", "1500");
    std::env::set_var("GATEWAY_RATE_LIMIT_PER_MINUTE", "240");
    std::env::set_var("GATEWAY_ANONYMOUS_RATE_LIMIT_PER_MINUTE", "60");

    let config = Config::from_env().expect("a complete environment");
    assert_eq!(config.port, 9090);
    assert_eq!(config.registry_url, "http://registry.internal:8081");
    assert_eq!(config.identity_url, "http://identity.internal:8083");
    assert_eq!(
        config.jwks_url,
        "https://project.supabase.co/auth/v1/.well-known/jwks.json"
    );
    assert_eq!(config.jwt_issuer, "https://project.supabase.co/auth/v1");
    assert_eq!(config.jwt_audience, "authenticated");
    assert_eq!(config.upstream_timeout_ms, 1500);
    assert_eq!(config.rate_limit_per_minute, 240);
    assert_eq!(config.anonymous_rate_limit_per_minute, 60);

    // The optional variables fall back to the documented defaults.
    for name in [
        "GATEWAY_PORT",
        "ASSET_REGISTRY_URL",
        "IDENTITY_URL",
        "UPSTREAM_TIMEOUT_MS",
        "GATEWAY_RATE_LIMIT_PER_MINUTE",
        "GATEWAY_ANONYMOUS_RATE_LIMIT_PER_MINUTE",
    ] {
        std::env::remove_var(name);
    }
    let config = Config::from_env().expect("the required variables are still set");
    assert_eq!(config.port, 8080);
    assert_eq!(config.registry_url, "http://asset-registry:8081");
    assert_eq!(config.identity_url, "http://identity-api:8083");
    assert_eq!(config.upstream_timeout_ms, 2000);
    assert_eq!(config.rate_limit_per_minute, 120);
    assert_eq!(config.anonymous_rate_limit_per_minute, 30);

    // Each required variable is named in its refusal.
    std::env::remove_var("JWT_JWKS_URL");
    let error = Config::from_env().expect_err("a missing JWKS URL is refused");
    assert!(
        error.to_string().contains("JWT_JWKS_URL must be set"),
        "{error}"
    );
    std::env::set_var("JWT_JWKS_URL", "https://project.supabase.co/jwks");

    std::env::remove_var("JWT_ISSUER");
    let error = Config::from_env().expect_err("a missing issuer is refused");
    assert!(
        error.to_string().contains("JWT_ISSUER must be set"),
        "{error}"
    );
    std::env::set_var("JWT_ISSUER", "https://project.supabase.co/auth/v1");

    std::env::remove_var("JWT_AUDIENCE");
    let error = Config::from_env().expect_err("a missing audience is refused");
    assert!(
        error.to_string().contains("JWT_AUDIENCE must be set"),
        "{error}"
    );
    std::env::set_var("JWT_AUDIENCE", "authenticated");

    // Each numeric variable names itself when it cannot be parsed.
    std::env::set_var("GATEWAY_PORT", "not-a-port");
    let error = Config::from_env().expect_err("a malformed port is refused");
    assert!(
        error
            .to_string()
            .contains("GATEWAY_PORT must be a valid port"),
        "{error}"
    );
    std::env::set_var("GATEWAY_PORT", "8080");

    std::env::set_var("UPSTREAM_TIMEOUT_MS", "soon");
    let error = Config::from_env().expect_err("a malformed timeout is refused");
    assert!(
        error
            .to_string()
            .contains("UPSTREAM_TIMEOUT_MS must be a number"),
        "{error}"
    );
    std::env::set_var("UPSTREAM_TIMEOUT_MS", "2000");

    std::env::set_var("GATEWAY_RATE_LIMIT_PER_MINUTE", "lots");
    let error = Config::from_env().expect_err("a malformed rate limit is refused");
    assert!(
        error
            .to_string()
            .contains("GATEWAY_RATE_LIMIT_PER_MINUTE must be a number"),
        "{error}"
    );
    std::env::set_var("GATEWAY_RATE_LIMIT_PER_MINUTE", "120");

    std::env::set_var("GATEWAY_ANONYMOUS_RATE_LIMIT_PER_MINUTE", "many");
    let error = Config::from_env().expect_err("a malformed anonymous rate limit is refused");
    assert!(
        error
            .to_string()
            .contains("GATEWAY_ANONYMOUS_RATE_LIMIT_PER_MINUTE must be a number"),
        "{error}"
    );

    clear_env();
}

fn dead_config(port: u16) -> Config {
    Config {
        port,
        registry_url: "http://127.0.0.1:1".to_string(),
        identity_url: "http://127.0.0.1:1".to_string(),
        jwks_url: "http://127.0.0.1:1/jwks".to_string(),
        jwt_issuer: "https://issuer.test".to_string(),
        jwt_audience: "authenticated".to_string(),
        upstream_timeout_ms: 200,
        rate_limit_per_minute: 60,
        anonymous_rate_limit_per_minute: 10,
    }
}

async fn get(state: gateway_http::AppState, uri: &str) -> (StatusCode, String) {
    request(state, uri, None).await
}

async fn request(
    state: gateway_http::AppState,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, String) {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router(state)
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// Serves the disposable test key as a JWKS document on a loopback port for the
/// life of the test process.
fn jwks_url() -> String {
    let key = test_key();
    let document = format!(
        r#"{{"keys":[{{"kid":"{}","kty":"RSA","n":"{}","e":"{}"}}]}}"#,
        key.kid, key.n, key.e
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = [0u8; 4096];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{document}",
                document.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://{addr}/jwks")
}

#[tokio::test]
async fn build_state_wires_the_service_name_routes_and_readiness() {
    let mut config = dead_config(8080);
    config.jwks_url = jwks_url();
    config.jwt_issuer = test_key().issuer.clone();
    config.jwt_audience = test_key().audience.clone();
    let state = gateway::build_state(&config).expect("the state is built");

    assert_eq!(state.service_name, "gateway");
    assert_eq!(gateway::service_name(), "gateway");

    let (status, body) = get(state.clone(), "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"service\":\"gateway\""), "{body}");

    let (status, body) = get(state.clone(), "/metrics").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("service_up"), "{body}");

    // Readiness follows the registry URL from the configuration.
    let (status, body) = get(state.clone(), "/readyz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        body.contains("not-ready") || body.contains("unavailable"),
        "{body}"
    );

    // An unauthenticated request never reaches routing.
    let (status, _) = get(state.clone(), "/v1/assets/resolve?symbol=BTC").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // With the forwarded JWT the declared route is matched and the dead upstream
    // is reported, rather than the gateway's own 404.
    let token = sign(test_key(), &TestClaims::valid(test_key(), "user"));
    let (status, body) =
        request(state.clone(), "/v1/assets/resolve?symbol=BTC", Some(&token)).await;
    assert!(
        status == StatusCode::BAD_GATEWAY
            || status == StatusCode::SERVICE_UNAVAILABLE
            || status == StatusCode::GATEWAY_TIMEOUT,
        "{status} {body}"
    );

    // An undeclared route is the gateway's own 404.
    let (status, body) = request(state, "/v1/not-declared", Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        body.contains("not-found") || body.contains("does not exist"),
        "{body}"
    );
}

/// Binds an ephemeral port, records it, and releases it for `run` to take.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

async fn raw_get(port: u16, path: &str) -> Option<String> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .ok()?;
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .ok()?;
        stream.flush().ok()?;
        let mut buffer = String::new();
        let _ = stream.read_to_string(&mut buffer);
        Some(buffer)
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn run_binds_the_configured_port_and_serves_the_gateway() {
    let port = free_port();
    let server = tokio::spawn(gateway::run(dead_config(port)));

    let mut response = None;
    for _ in 0..100 {
        if let Some(text) = raw_get(port, "/healthz").await {
            if text.contains("200") {
                response = Some(text);
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    server.abort();

    let response = response.expect("the gateway answered /healthz on its bound port");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("\"service\":\"gateway\""), "{response}");
}

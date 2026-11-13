//! Upstream HTTP client tests against a local axum server.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use gateway_application::ports::{ForwardRequest, UpstreamClient, UpstreamError};
use gateway_domain::{Method, ServiceName};
use gateway_infrastructure::upstream::ReqwestUpstreamClient;
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
struct Recorded {
    method: String,
    uri: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

async fn record(
    State(recorded): State<Arc<Mutex<Option<Recorded>>>>,
    request: Request<Body>,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1_000_000)
        .await
        .expect("read body")
        .to_vec();
    let headers = parts
        .headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    *recorded.lock().unwrap() = Some(Recorded {
        method: parts.method.to_string(),
        uri: parts.uri.to_string(),
        headers,
        body: bytes,
    });
    (
        StatusCode::CREATED,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"upstream":true}"#,
    )
        .into_response()
}

async fn start_server() -> (String, Arc<Mutex<Option<Recorded>>>) {
    let recorded = Arc::new(Mutex::new(None));
    let app = Router::new().fallback(record).with_state(recorded.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve test upstream");
    });
    (format!("http://{address}"), recorded)
}

fn forward(path_and_query: &str, method: Method) -> ForwardRequest {
    ForwardRequest {
        service: ServiceName::registry(),
        method,
        path_and_query: path_and_query.to_string(),
        headers: vec![
            (
                "authorization".to_string(),
                "Bearer abc.def.ghi".to_string(),
            ),
            ("connection".to_string(), "keep-alive".to_string()),
            ("x-custom".to_string(), "custom-value".to_string()),
        ],
        body: b"{\"hello\":\"world\"}".to_vec(),
        correlation_id: "corr-upstream".to_string(),
    }
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

#[tokio::test]
async fn forwards_method_path_query_body_and_headers() {
    let (base_url, recorded) = start_server().await;
    let client = ReqwestUpstreamClient::new(&base_url, 1_000);

    let response = client
        .forward(forward("/v1/assets/resolve?chain=ethereum", Method::Get))
        .await
        .expect("upstream answers");

    assert_eq!(response.status, 201);
    assert_eq!(response.body, br#"{"upstream":true}"#);
    assert_eq!(response.content_type.as_deref(), Some("application/json"));

    let recorded = recorded.lock().unwrap().clone().expect("recorded request");
    assert_eq!(recorded.method, "GET");
    assert_eq!(recorded.uri, "/v1/assets/resolve?chain=ethereum");
    assert_eq!(recorded.body, b"{\"hello\":\"world\"}");
    assert_eq!(
        header_value(&recorded.headers, "authorization"),
        Some("Bearer abc.def.ghi")
    );
    assert_eq!(
        header_value(&recorded.headers, "x-custom"),
        Some("custom-value")
    );
    assert_eq!(
        header_value(&recorded.headers, "x-correlation-id"),
        Some("corr-upstream")
    );
}

#[tokio::test]
async fn hop_by_hop_headers_are_not_forwarded() {
    let (base_url, recorded) = start_server().await;
    let client = ReqwestUpstreamClient::new(&base_url, 1_000);

    client
        .forward(forward("/v1/assets/resolve", Method::Get))
        .await
        .expect("upstream answers");

    let recorded = recorded.lock().unwrap().clone().expect("recorded request");
    assert!(header_value(&recorded.headers, "connection").is_none());
}

#[tokio::test]
async fn a_slow_upstream_maps_to_timeout() {
    async fn slow() -> Response {
        tokio::time::sleep(Duration::from_millis(250)).await;
        StatusCode::OK.into_response()
    }
    let app = Router::new().fallback(slow);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let client = ReqwestUpstreamClient::new(format!("http://{address}"), 50);
    let result = client
        .forward(forward("/v1/assets/resolve", Method::Get))
        .await;
    assert!(matches!(result, Err(UpstreamError::Timeout)));
}

#[tokio::test]
async fn an_unreachable_upstream_maps_to_unavailable() {
    // Reserve a port, then drop the listener so nothing is listening.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    drop(listener);

    // Windows takes about two seconds to refuse a connection to a closed local
    // port, so allow well over that for the refusal to be observed.
    let client = ReqwestUpstreamClient::new(format!("http://{address}"), 5_000);
    let result = client
        .forward(forward("/v1/assets/resolve", Method::Get))
        .await;
    assert!(matches!(result, Err(UpstreamError::Unavailable(_))));
}

#[tokio::test]
async fn each_route_is_forwarded_to_its_own_service_and_an_unknown_service_is_refused() {
    let (registry_url, registry_seen) = start_server().await;
    let (identity_url, identity_seen) = start_server().await;
    let client = ReqwestUpstreamClient::with_services(
        std::collections::BTreeMap::from([
            ("asset-registry".to_string(), registry_url),
            ("identity".to_string(), identity_url),
        ]),
        1_000,
    );

    client
        .forward(forward("/v1/assets/resolve", Method::Get))
        .await
        .expect("the registry answers");
    assert!(registry_seen.lock().unwrap().is_some());
    assert!(
        identity_seen.lock().unwrap().is_none(),
        "the registry request never reaches Identity"
    );

    let mut identity_request = forward("/v1/principals/abc", Method::Get);
    identity_request.service = ServiceName::identity();
    client
        .forward(identity_request)
        .await
        .expect("identity answers");
    let seen = identity_seen.lock().unwrap().clone().expect("recorded");
    assert_eq!(seen.uri, "/v1/principals/abc");

    let mut unknown = forward("/v1/principals/abc", Method::Get);
    unknown.service = ServiceName::new("portfolio").unwrap();
    let error = client
        .forward(unknown)
        .await
        .expect_err("an undeclared service has no upstream");
    assert!(matches!(error, UpstreamError::Unavailable(_)));
}

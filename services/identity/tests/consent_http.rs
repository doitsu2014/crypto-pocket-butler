//! U4 HTTP tests: `GET|PUT /v1/me/preferences`, the consent routes, and the C6
//! internal consent-state route.

mod common;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use common::*;
use http_body_util::BodyExt;
use identity_domain::preferences::{BaseCurrency, Locale, TimeZone};
use identity_domain::{ProviderProfile, UserId};
use identity_http::router;
use identity_http::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn send(
    state: &AppState,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = router(state.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, body)
}

async fn send_auth(
    state: &AppState,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    let authorization = format!("Bearer {token}");
    send(
        state,
        method,
        uri,
        &[("authorization", authorization.as_str())],
        body,
    )
    .await
}

fn http_harness() -> (Harness, UserId) {
    let harness = harness();
    let user = active_user();
    harness.users.seed(user.clone(), USER_SUBJECT);
    harness.user_lookup.seed(
        user.id(),
        ProviderProfile::new(
            Locale::En,
            BaseCurrency::parse("USD").unwrap(),
            TimeZone::parse("UTC").unwrap(),
        ),
    );
    (harness, user.id())
}

#[tokio::test]
async fn preferences_read_before_the_first_write_returns_defaults() {
    let (harness, _user_id) = http_harness();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/me/preferences",
        "user-token",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["locale"]["value"], "en");
    assert_eq!(body["locale"]["source"], "default");
    assert_eq!(body["baseCurrency"]["value"], "USD");
    assert_eq!(body["notifications"]["email"]["value"], false);
    assert_eq!(body["notifications"]["email"]["source"], "default");
}

#[tokio::test]
async fn preferences_write_applies_only_the_supplied_field() {
    let (harness, _user_id) = http_harness();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PUT,
        "/v1/me/preferences",
        "user-token",
        Some(json!({ "locale": "vi" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["locale"]["value"], "vi");
    assert_eq!(body["locale"]["source"], "user");
    assert_eq!(body["baseCurrency"]["value"], "USD");
    assert_eq!(
        body["baseCurrency"]["source"], "default",
        "an unsupplied field keeps its default"
    );
}

#[tokio::test]
async fn preferences_write_refuses_an_out_of_set_currency_and_timezone() {
    let (harness, _user_id) = http_harness();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PUT,
        "/v1/me/preferences",
        "user-token",
        Some(json!({ "baseCurrency": "CAD" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["errors"][0]["field"], "baseCurrency");

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PUT,
        "/v1/me/preferences",
        "user-token",
        Some(json!({ "timezone": "Not/AZone" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["errors"][0]["field"], "timezone");
}

#[tokio::test]
async fn preferences_write_refuses_an_unknown_notification_channel() {
    let (harness, _user_id) = http_harness();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PUT,
        "/v1/me/preferences",
        "user-token",
        Some(json!({ "notifications": { "sms": true } })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["errors"][0]["field"], "notifications.sms");
}

#[tokio::test]
async fn consent_state_lists_every_kind_and_the_missing_acceptances() {
    let (harness, user_id) = http_harness();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/me/consent/state",
        "user-token",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["userId"], user_id.to_string());
    assert_eq!(body["kinds"]["terms"], "none");
    assert_eq!(body["kinds"]["marketing"], "none");
    assert_eq!(body["missing"], json!(["terms", "risk", "age"]));
}

#[tokio::test]
async fn consent_grant_records_the_event_and_the_state_reflects_it() {
    let (harness, _user_id) = http_harness();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/me/consent",
        "user-token",
        Some(json!({ "kind": "terms", "policyVersion": "2026-11-01.1" })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["kind"], "terms");
    assert_eq!(body["state"], "granted");

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/me/consent/state",
        "user-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["kinds"]["terms"], "granted");
    assert_eq!(body["missing"], json!(["risk", "age"]));
}

#[tokio::test]
async fn consent_grant_refuses_an_unknown_kind_and_a_missing_policy_version() {
    let (harness, _user_id) = http_harness();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/me/consent",
        "user-token",
        Some(json!({ "kind": "disclosures", "policyVersion": "2026-11-01.1" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["errors"][0]["field"], "kind");

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/me/consent",
        "user-token",
        Some(json!({ "kind": "terms" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["errors"][0]["field"], "policyVersion");
}

#[tokio::test]
async fn consent_withdraw_is_idempotent_and_a_never_granted_kind_conflicts() {
    let (harness, _user_id) = http_harness();
    let _ = send_auth(
        &harness.state,
        Method::POST,
        "/v1/me/consent",
        "user-token",
        Some(json!({ "kind": "terms", "policyVersion": "2026-11-01.1" })),
    )
    .await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/me/consent/terms/withdraw",
        "user-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["state"], "withdrawn");
    assert_eq!(body["changed"], true);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/me/consent/terms/withdraw",
        "user-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "a second withdrawal is a no-op");
    assert_eq!(body["changed"], false);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/me/consent/risk/withdraw",
        "user-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(
        body["detail"].as_str().unwrap_or("").contains("risk"),
        "the conflict names the kind"
    );
}

#[tokio::test]
async fn consent_history_returns_newest_first_and_pages_by_cursor() {
    let (harness, _user_id) = http_harness();
    for kind in ["terms", "risk"] {
        let _ = send_auth(
            &harness.state,
            Method::POST,
            "/v1/me/consent",
            "user-token",
            Some(json!({ "kind": kind, "policyVersion": "2026-11-01.1" })),
        )
        .await;
    }

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/me/consent?limit=1",
        "user-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["events"].as_array().unwrap().len(), 1);
    assert_eq!(body["events"][0]["kind"], "risk", "newest first");
    let cursor = body["nextCursor"]
        .as_str()
        .expect("a cursor continues the page")
        .to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/me/consent?limit=1&before={cursor}"),
        "user-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["events"][0]["kind"], "terms");
}

#[tokio::test]
async fn the_c6_consent_state_route_answers_for_the_token_user_only() {
    let (harness, user_id) = http_harness();
    let other = active_user();
    harness.users.seed(other.clone(), OTHER_SUBJECT);
    harness.user_lookup.seed(
        other.id(),
        ProviderProfile::new(
            Locale::En,
            BaseCurrency::parse("USD").unwrap(),
            TimeZone::parse("UTC").unwrap(),
        ),
    );
    let uri = format!("/internal/users/{user_id}/consent/state");
    let authorization = "Bearer user-token";
    let other_authorization = "Bearer other-token";

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[
            ("authorization", authorization),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["userId"], user_id.to_string());

    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[
            ("authorization", authorization),
            ("x-service-token", "portfolio-token"),
        ],
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "only the source-connection service may ask"
    );

    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[
            ("authorization", other_authorization),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "another user's state is indistinguishable from absent"
    );

    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[("authorization", authorization)],
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a service token is required"
    );
}

#[tokio::test]
async fn the_c6_declared_preferences_and_consent_routes_work() {
    let (harness, user_id) = http_harness();
    let preferences = format!("/internal/users/{user_id}/preferences");
    let consent = format!("/internal/users/{user_id}/consent");
    let authorization = "Bearer user-token";
    let service = ("x-service-token", "account-token");

    let (status, _, body) = send(
        &harness.state,
        Method::PUT,
        &preferences,
        &[("authorization", authorization), service],
        Some(json!({ "locale": "vi" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["locale"]["value"], "vi");
    assert_eq!(body["locale"]["source"], "user");

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &preferences,
        &[("authorization", authorization), service],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["locale"]["value"], "vi");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &consent,
        &[("authorization", authorization), service],
        Some(json!({ "kind": "terms", "policyVersion": "2026-11-01.1" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["state"], "granted");

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &consent,
        &[("authorization", authorization), service],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["events"].as_array().unwrap().len(), 1);
    assert_eq!(body["events"][0]["kind"], "terms");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &consent,
        &[("authorization", authorization), service],
        Some(json!({ "kind": "terms", "action": "withdrawn" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["state"], "withdrawn");
    assert_eq!(body["changed"], true);

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &consent,
        &[
            ("authorization", authorization),
            ("x-service-token", "portfolio-token"),
        ],
        Some(json!({ "kind": "risk", "policyVersion": "v1" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let other = active_user();
    harness.users.seed(other.clone(), OTHER_SUBJECT);
    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &format!("/internal/users/{}/preferences", other.id()),
        &[("authorization", authorization), service],
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "another user's id is indistinguishable from absent"
    );
}

#[tokio::test]
async fn a_consent_state_read_failure_is_a_503() {
    let (harness, _user_id) = http_harness();
    harness
        .consent_events
        .fail_next(identity_application::ports::RepositoryError::Unavailable(
            "down".to_string(),
        ));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/me/consent/state",
        "user-token",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["status"], 503);
}

#[tokio::test]
async fn the_new_consent_routes_require_a_jwt() {
    let (harness, _user_id) = http_harness();

    for uri in [
        "/v1/me/preferences",
        "/v1/me/consent/state",
        "/v1/me/consent",
    ] {
        let (status, _, _) = send(&harness.state, Method::GET, uri, &[], None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
}

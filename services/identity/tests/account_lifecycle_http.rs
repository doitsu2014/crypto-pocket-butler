//! HTTP boundary tests for the account lifecycle (C7) and the tombstone answer
//! C3 gained: status codes, RFC 7807 bodies, `recoveryEndsAt`, and the 403/409/
//! 410/422 catalogue the functional spec records.

mod common;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use common::{admin_claims, claims, fixed_now, harness, USER_SUBJECT};
use http_body_util::BodyExt;
use identity_application::ports::UserRepository;
use identity_application::user_registry::subject_hash;
use identity_http::router;
use identity_http::state::AppState;
use serde_json::Value;
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
    (
        status,
        headers,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// Provisions the regular user and the admin through the public principal path.
async fn seed(state: &AppState) -> (String, String) {
    let owner = state
        .principal
        .resolve(claims(), "corr-http-owner")
        .await
        .expect("the owner is provisioned");
    let admin = state
        .principal
        .resolve(admin_claims(), "corr-http-admin")
        .await
        .expect("the admin is provisioned");
    (owner.user_id().to_string(), admin.user_id().to_string())
}

#[tokio::test]
async fn an_admin_disables_and_restores_through_c7_and_a_non_admin_is_forbidden() {
    let harness = harness();
    let (owner_id, _admin_id) = seed(&harness.state).await;
    let admin = bearer("admin-token");
    // A second, active, non-admin account so the refusal is the role, not the
    // disabled status of the target.
    let non_admin = bearer("other-token");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/disable"),
        &[
            ("authorization", admin.as_str()),
            ("x-service-token", "account-token"),
        ],
        Some(serde_json::json!({ "reason": "account abuse" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["status"], "disabled");
    assert_eq!(body["userId"], owner_id);

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/disable"),
        &[
            ("authorization", non_admin.as_str()),
            ("x-service-token", "account-token"),
        ],
        Some(serde_json::json!({ "reason": "self service" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{}", body);
    assert_eq!(body["type"], "https://errors.cpb/identity/forbidden");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/restore"),
        &[
            ("authorization", admin.as_str()),
            ("x-service-token", "account-token"),
        ],
        Some(serde_json::json!({ "reason": "appeal upheld" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn a_blank_reason_is_422_naming_the_field_and_an_illegal_edge_is_409() {
    let harness = harness();
    let (owner_id, _admin_id) = seed(&harness.state).await;
    let admin = bearer("admin-token");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/disable"),
        &[
            ("authorization", admin.as_str()),
            ("x-service-token", "account-token"),
        ],
        Some(serde_json::json!({ "reason": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{}", body);
    assert_eq!(body["errors"][0]["field"], "reason");

    for _ in 0..2 {
        let _ = send(
            &harness.state,
            Method::POST,
            &format!("/internal/users/{owner_id}/disable"),
            &[
                ("authorization", admin.as_str()),
                ("x-service-token", "account-token"),
            ],
            Some(serde_json::json!({ "reason": "abuse" })),
        )
        .await;
    }
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/disable"),
        &[
            ("authorization", admin.as_str()),
            ("x-service-token", "account-token"),
        ],
        Some(serde_json::json!({ "reason": "again" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["type"], "https://errors.cpb/identity/conflict");
}

#[tokio::test]
async fn an_admin_disables_and_restores_on_the_gateway_facing_admin_paths() {
    let harness = harness();
    let (owner_id, _admin_id) = seed(&harness.state).await;
    let admin = bearer("admin-token");

    // The forwarded JWT alone authenticates: no service token is supplied.
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{owner_id}/disable"),
        &[("authorization", admin.as_str())],
        Some(serde_json::json!({ "reason": "account abuse" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["status"], "disabled");
    assert_eq!(body["userId"], owner_id);

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{owner_id}/restore"),
        &[("authorization", admin.as_str())],
        Some(serde_json::json!({ "reason": "appeal upheld" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn the_admin_status_paths_answer_403_404_409_and_422() {
    let harness = harness();
    let (owner_id, _admin_id) = seed(&harness.state).await;
    let admin = bearer("admin-token");
    let non_admin = bearer("user-token");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{owner_id}/disable"),
        &[("authorization", non_admin.as_str())],
        Some(serde_json::json!({ "reason": "self service" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{}", body);
    assert_eq!(body["type"], "https://errors.cpb/identity/forbidden");

    let unknown = identity_domain::UserId::generate().to_string();
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{unknown}/disable"),
        &[("authorization", admin.as_str())],
        Some(serde_json::json!({ "reason": "account abuse" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{}", body);

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{owner_id}/disable"),
        &[("authorization", admin.as_str())],
        Some(serde_json::json!({ "reason": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{}", body);
    assert_eq!(body["errors"][0]["field"], "reason");

    for _ in 0..2 {
        let _ = send(
            &harness.state,
            Method::POST,
            &format!("/v1/admin/users/{owner_id}/disable"),
            &[("authorization", admin.as_str())],
            Some(serde_json::json!({ "reason": "abuse" })),
        )
        .await;
    }
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{owner_id}/disable"),
        &[("authorization", admin.as_str())],
        Some(serde_json::json!({ "reason": "again" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["type"], "https://errors.cpb/identity/conflict");
}

#[tokio::test]
async fn the_owner_opens_a_deletion_request_and_a_second_one_is_409() {
    let harness = harness();
    let (owner_id, _admin_id) = seed(&harness.state).await;
    let owner = bearer("user-token");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/deletion-requests"),
        &[
            ("authorization", owner.as_str()),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", body);
    let reported = body["recoveryEndsAt"].as_str().unwrap();
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(reported)
            .unwrap()
            .with_timezone(&chrono::Utc),
        identity_domain::recoverable_until(fixed_now())
    );

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/deletion-requests"),
        &[
            ("authorization", owner.as_str()),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
}

#[tokio::test]
async fn the_owner_restores_inside_the_window_and_a_late_restore_is_410() {
    let harness = harness();
    let (owner_id, _admin_id) = seed(&harness.state).await;
    let owner = bearer("user-token");

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/deletion-requests"),
        &[
            ("authorization", owner.as_str()),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    harness.clock.set(fixed_now() + chrono::Duration::days(1));
    let (status, _, body) = send(
        &harness.state,
        Method::DELETE,
        &format!("/internal/users/{owner_id}/deletion-requests"),
        &[
            ("authorization", owner.as_str()),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert!(body["cancelledAt"].is_string());

    // Another user's id answers 404, so the route confirms nothing.
    let (status, _, _) = send(
        &harness.state,
        Method::DELETE,
        "/internal/users/00000000-0000-4000-8000-000000000000/deletion-requests",
        &[
            ("authorization", owner.as_str()),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A new request restored after its deadline is 410.
    harness.clock.set(fixed_now() + chrono::Duration::days(32));
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &format!("/internal/users/{owner_id}/deletion-requests"),
        &[
            ("authorization", owner.as_str()),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    harness.clock.set(fixed_now() + chrono::Duration::days(63));
    let (status, _, body) = send(
        &harness.state,
        Method::DELETE,
        &format!("/internal/users/{owner_id}/deletion-requests"),
        &[
            ("authorization", owner.as_str()),
            ("x-service-token", "account-token"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::GONE, "{}", body);
    assert_eq!(body["type"], "https://errors.cpb/identity/gone");
}

#[tokio::test]
async fn an_erased_subject_answers_410_on_c3_instead_of_404() {
    let harness = harness();
    // A deleted row with its tombstone in place: the subject hash resolves to
    // the erased record, which C3 must answer `410`, never `404`.
    let base = common::active_user();
    let erased = identity_domain::User::from_stored(
        base.id(),
        base.role(),
        identity_domain::UserStatus::Deleted,
        None,
        false,
        None,
        false,
        false,
        base.locale(),
        base.base_currency(),
        base.timezone(),
        base.created_at(),
        fixed_now(),
        Some(fixed_now()),
    );
    harness.users.seed(erased.clone(), USER_SUBJECT);
    harness.users.anonymise(&erased).await.unwrap();

    let hash = subject_hash(USER_SUBJECT);
    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/internal/users/by-subject/{hash}"),
        &[("authorization", "Bearer gateway-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::GONE, "{}", body);
    assert_eq!(body["type"], "https://errors.cpb/identity/gone");
}

#[tokio::test]
async fn the_legacy_deletion_surface_reports_the_window_and_refuses_a_second_request() {
    let harness = harness();
    let (_owner_id, _admin_id) = seed(&harness.state).await;
    let owner = bearer("user-token");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/privacy/deletions",
        &[("authorization", owner.as_str())],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", body);
    assert!(body["recoveryEndsAt"].is_string());
    let deletion_id = body["deletionId"].as_str().unwrap().to_string();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/privacy/deletions/{deletion_id}"),
        &[("authorization", owner.as_str())],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["status"], "requested");

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/privacy/deletions",
        &[("authorization", owner.as_str())],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

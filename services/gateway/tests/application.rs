//! Gateway use-case tests: JWT enforcement, deny-by-default routing, admin
//! gating, correlation propagation, bounded retries, and error mapping.

mod common;

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use common::fakes::{incoming, FakeAudit, FakeUpstream, FakeValidator, FixedClock};
use gateway_application::authorize::AuthorizeAndRoute;
use gateway_application::ports::{Clock, ForwardRequest, UpstreamError, UpstreamResponse};
use gateway_domain::{GatewayError, Method, RouteTable};

fn at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap()
}

fn build(
    validator: FakeValidator,
    upstream: Arc<FakeUpstream>,
    audit: Arc<FakeAudit>,
) -> AuthorizeAndRoute {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(at()));
    AuthorizeAndRoute::new(
        Arc::new(validator),
        upstream,
        audit,
        clock,
        RouteTable::default_registry_routes(),
    )
}

fn header<'a>(request: &'a ForwardRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

#[tokio::test]
async fn a_declared_route_is_forwarded_with_the_jwt_unchanged() {
    let upstream = Arc::new(FakeUpstream::ok());
    let use_case = build(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    let response = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            Some("corr-1"),
        ))
        .await
        .expect("declared route");

    assert_eq!(response.status, 200);
    assert_eq!(response.body, br#"{"ok":true}"#);
    assert_eq!(response.correlation_id, "corr-1");
    let forwarded = upstream.last_request();
    assert_eq!(forwarded.service.as_str(), "asset-registry");
    assert_eq!(forwarded.path_and_query, "/v1/assets/resolve");
    assert_eq!(
        header(&forwarded, "authorization"),
        Some("Bearer valid-user-token"),
        "the user JWT is forwarded unchanged"
    );
    assert_eq!(header(&forwarded, "x-correlation-id"), Some("corr-1"));
}

#[tokio::test]
async fn a_missing_token_is_rejected_before_routing_and_audited() {
    let upstream = Arc::new(FakeUpstream::ok());
    let audit = Arc::new(FakeAudit::default());
    let use_case = build(FakeValidator::user(), upstream.clone(), audit.clone());

    let result = use_case
        .execute(incoming(Method::Get, "/v1/assets/resolve", None, None))
        .await;

    assert!(matches!(result, Err(GatewayError::Unauthorized(_))));
    assert_eq!(upstream.calls(), 0);
    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, "gateway.denied");
    assert_eq!(events[0].actor, "anonymous");
}

#[tokio::test]
async fn an_invalid_token_is_rejected_and_audited() {
    let upstream = Arc::new(FakeUpstream::ok());
    let audit = Arc::new(FakeAudit::default());
    let use_case = build(FakeValidator::user(), upstream.clone(), audit.clone());

    let result = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("forged-token"),
            None,
        ))
        .await;

    assert!(matches!(result, Err(GatewayError::Unauthorized(_))));
    assert_eq!(upstream.calls(), 0);
    assert_eq!(audit.events().len(), 1);
}

#[tokio::test]
async fn jwks_unavailability_fails_closed_with_503() {
    let upstream = Arc::new(FakeUpstream::ok());
    let use_case = build(
        FakeValidator::unavailable(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    let result = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            None,
        ))
        .await;

    assert!(matches!(result, Err(GatewayError::ServiceUnavailable(_))));
    assert_eq!(upstream.calls(), 0);
}

#[tokio::test]
async fn an_undeclared_route_is_denied_by_default_and_audited() {
    let upstream = Arc::new(FakeUpstream::ok());
    let audit = Arc::new(FakeAudit::default());
    let use_case = build(FakeValidator::user(), upstream.clone(), audit.clone());

    let result = use_case
        .execute(incoming(
            Method::Get,
            "/v1/secret/export",
            Some("valid-user-token"),
            None,
        ))
        .await;

    assert!(matches!(result, Err(GatewayError::NotFound(_))));
    assert_eq!(upstream.calls(), 0);
    assert_eq!(audit.events().len(), 1);
    assert_eq!(audit.events()[0].actor, "user-1");
}

#[tokio::test]
async fn a_user_cannot_use_an_admin_route_but_an_admin_can() {
    let upstream = Arc::new(FakeUpstream::ok());
    let audit = Arc::new(FakeAudit::default());
    let use_case = build(FakeValidator::user(), upstream.clone(), audit.clone());

    let denied = use_case
        .execute(incoming(
            Method::Post,
            "/v1/admin/assets",
            Some("valid-user-token"),
            None,
        ))
        .await;
    assert!(matches!(denied, Err(GatewayError::Forbidden(_))));
    assert_eq!(upstream.calls(), 0);
    assert_eq!(audit.events().len(), 1);

    let admin_upstream = Arc::new(FakeUpstream::ok());
    let admin_use_case = build(
        FakeValidator::admin(),
        admin_upstream.clone(),
        Arc::new(FakeAudit::default()),
    );
    let allowed = admin_use_case
        .execute(incoming(
            Method::Post,
            "/v1/admin/assets",
            Some("valid-admin-token"),
            None,
        ))
        .await;
    assert!(allowed.is_ok());
    assert_eq!(admin_upstream.calls(), 1);
}

#[tokio::test]
async fn a_missing_correlation_id_is_generated_and_forwarded() {
    let upstream = Arc::new(FakeUpstream::ok());
    let use_case = build(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    let response = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            None,
        ))
        .await
        .unwrap();

    assert_eq!(response.correlation_id.len(), 36);
    assert!(response.correlation_id.contains('-'));
    assert_eq!(
        header(&upstream.last_request(), "x-correlation-id"),
        Some(response.correlation_id.as_str())
    );
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused_with_400() {
    let use_case = build(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
    );

    let result = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            Some("bad id"),
        ))
        .await;

    assert!(matches!(result, Err(GatewayError::BadRequest(_))));
}

#[tokio::test]
async fn an_upstream_timeout_maps_to_504_after_bounded_retries() {
    let upstream = Arc::new(FakeUpstream::with_responses(vec![
        Err(UpstreamError::Timeout),
        Err(UpstreamError::Timeout),
        Err(UpstreamError::Timeout),
    ]));
    let use_case = build(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    let result = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            None,
        ))
        .await;

    assert!(matches!(result, Err(GatewayError::GatewayTimeout(_))));
    assert_eq!(
        upstream.calls(),
        3,
        "bounded retries for an idempotent request"
    );
}

#[tokio::test]
async fn an_idempotent_request_is_retried_and_can_succeed() {
    let upstream = Arc::new(FakeUpstream::with_responses(vec![
        Err(UpstreamError::Unavailable("connection reset".to_string())),
        Ok(UpstreamResponse {
            status: 200,
            headers: Vec::new(),
            body: b"ok".to_vec(),
            content_type: None,
        }),
    ]));
    let use_case = build(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    let response = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            None,
        ))
        .await
        .expect("the retry succeeds");

    assert_eq!(response.status, 200);
    assert_eq!(upstream.calls(), 2);
}

#[tokio::test]
async fn a_mutation_is_not_retried_and_maps_to_502() {
    let upstream = Arc::new(FakeUpstream::with_responses(vec![
        Err(UpstreamError::Unavailable("connection reset".to_string())),
        Ok(UpstreamResponse {
            status: 200,
            headers: Vec::new(),
            body: b"should not be reached".to_vec(),
            content_type: None,
        }),
    ]));
    let use_case = build(
        FakeValidator::admin(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    let result = use_case
        .execute(incoming(
            Method::Post,
            "/v1/admin/assets",
            Some("valid-admin-token"),
            None,
        ))
        .await;

    assert!(matches!(result, Err(GatewayError::BadGateway(_))));
    assert_eq!(upstream.calls(), 1);
}

#[tokio::test]
async fn a_mutation_carries_an_idempotency_key_and_preserves_a_caller_supplied_one() {
    let upstream = Arc::new(FakeUpstream::ok());
    let use_case = build(
        FakeValidator::admin(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    use_case
        .execute(incoming(
            Method::Post,
            "/v1/admin/assets",
            Some("valid-admin-token"),
            None,
        ))
        .await
        .unwrap();
    let generated = header(&upstream.last_request(), "idempotency-key")
        .expect("generated idempotency key")
        .to_string();
    assert_eq!(generated.len(), 36);

    let mut request = incoming(
        Method::Post,
        "/v1/admin/assets",
        Some("valid-admin-token"),
        None,
    );
    request
        .headers
        .push(("idempotency-key".to_string(), "caller-key-1".to_string()));
    use_case.execute(request).await.unwrap();
    assert_eq!(
        header(&upstream.last_request(), "idempotency-key"),
        Some("caller-key-1")
    );
}

#[tokio::test]
async fn a_read_does_not_carry_an_idempotency_key() {
    let upstream = Arc::new(FakeUpstream::ok());
    let use_case = build(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            None,
        ))
        .await
        .unwrap();
    assert!(header(&upstream.last_request(), "idempotency-key").is_none());
}

#[tokio::test]
async fn an_upstream_rejection_maps_to_502_without_retry() {
    let upstream = Arc::new(FakeUpstream::with_responses(vec![Err(
        UpstreamError::Rejected("bad request".to_string()),
    )]));
    let use_case = build(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
    );

    let result = use_case
        .execute(incoming(
            Method::Get,
            "/v1/assets/resolve",
            Some("valid-user-token"),
            None,
        ))
        .await;

    assert!(matches!(result, Err(GatewayError::BadGateway(_))));
    assert_eq!(upstream.calls(), 1);
}

//! Gateway domain tests: deny-by-default routing, correlation ids, and error mapping.

use gateway_domain::{
    CorrelationId, GatewayError, Method, PathPattern, Role, Route, RouteTable, ServiceName,
};

#[test]
fn the_route_table_matches_a_declared_read_route() {
    let table = RouteTable::default_registry_routes();
    let route = table
        .match_route(Method::Get, "/v1/assets/resolve")
        .expect("declared route");
    assert_eq!(route.service.as_str(), "asset-registry");
    assert!(!route.requires_admin);
}

#[test]
fn an_undeclared_route_is_denied_by_default() {
    let table = RouteTable::default_registry_routes();
    assert!(table
        .match_route(Method::Get, "/v1/secret/export")
        .is_none());
    assert!(table
        .match_route(Method::Delete, "/v1/assets/resolve")
        .is_none());
}

#[test]
fn a_parameter_segment_matches_exactly_one_path_segment() {
    let table = RouteTable::default_registry_routes();
    let route = table
        .match_route(Method::Get, "/v1/assets/asset-usdc/price")
        .expect("parameter route");
    assert_eq!(route.service.as_str(), "asset-registry");
    assert!(table
        .match_route(Method::Get, "/v1/assets/asset-usdc/price/extra")
        .is_none());
    assert!(table
        .match_route(Method::Get, "/v1/assets//price")
        .is_none());
}

#[test]
fn registry_administration_routes_require_the_admin_role() {
    let table = RouteTable::default_registry_routes();
    for (method, path) in [
        (Method::Post, "/v1/admin/assets"),
        (Method::Post, "/v1/admin/token-contracts"),
        (Method::Put, "/v1/admin/price-sources"),
        (Method::Patch, "/v1/admin/price-sources/feed-a/enabled"),
        (Method::Post, "/v1/admin/observations/obs-1/resolve"),
        (Method::Post, "/v1/admin/observations/obs-1/ignore"),
    ] {
        let route = table.match_route(method, path).expect("admin route");
        assert!(route.requires_admin, "{method:?} {path}");
    }
}

#[test]
fn correlation_ids_are_bounded_and_character_checked() {
    assert_eq!(
        CorrelationId::parse("abc-123_X.Y:Z").unwrap().as_str(),
        "abc-123_X.Y:Z"
    );
    assert!(matches!(
        CorrelationId::parse(""),
        Err(GatewayError::BadRequest(_))
    ));
    assert!(matches!(
        CorrelationId::parse(&"a".repeat(129)),
        Err(GatewayError::BadRequest(_))
    ));
    assert!(matches!(
        CorrelationId::parse("has a space"),
        Err(GatewayError::BadRequest(_))
    ));
}

#[test]
fn gateway_errors_map_to_http_status_and_problem_type() {
    let cases = [
        (
            GatewayError::Unauthorized(String::new()),
            401,
            "unauthorized",
        ),
        (GatewayError::Forbidden(String::new()), 403, "forbidden"),
        (GatewayError::NotFound(String::new()), 404, "not-found"),
        (GatewayError::BadRequest(String::new()), 400, "bad-request"),
        (
            GatewayError::ServiceUnavailable(String::new()),
            503,
            "service-unavailable",
        ),
        (GatewayError::BadGateway(String::new()), 502, "bad-gateway"),
        (
            GatewayError::GatewayTimeout(String::new()),
            504,
            "gateway-timeout",
        ),
    ];
    for (error, status, problem_type) in cases {
        assert_eq!(error.status(), status);
        assert_eq!(error.problem_type(), problem_type);
    }
}

#[test]
fn idempotent_methods_are_recognized_for_bounded_retries() {
    assert!(Method::Get.is_idempotent());
    assert!(Method::Head.is_idempotent());
    assert!(Method::Options.is_idempotent());
    assert!(Method::Put.is_idempotent());
    assert!(Method::Delete.is_idempotent());
    assert!(!Method::Post.is_idempotent());
    assert!(!Method::Patch.is_idempotent());
}

#[test]
fn path_patterns_reject_malformed_declarations() {
    assert!(PathPattern::parse("v1/assets").is_err());
    assert!(PathPattern::parse("/v1//assets").is_err());
    assert!(PathPattern::parse("/v1/{}/assets").is_err());
    assert!(PathPattern::parse("/v1/{id}/assets").is_ok());
}

#[test]
fn service_names_reject_blank_values() {
    assert!(ServiceName::new("  ").is_err());
    assert_eq!(ServiceName::registry().as_str(), "asset-registry");
    assert_eq!(ServiceName::identity().as_str(), "identity");
}

#[test]
fn the_identity_routes_are_declared_with_the_admin_search_gated() {
    let table = RouteTable::default_registry_routes();

    for (method, path) in [
        (
            Method::Get,
            "/v1/principals/11111111-1111-4111-8111-111111111111",
        ),
        (
            Method::Get,
            "/v1/users/11111111-1111-4111-8111-111111111111",
        ),
    ] {
        let route = table.match_route(method, path).expect("identity route");
        assert_eq!(route.service.as_str(), "identity");
        assert!(!route.requires_admin);
    }

    let search = table
        .match_route(Method::Get, "/v1/admin/users")
        .expect("admin search route");
    assert_eq!(search.service.as_str(), "identity");
    assert!(
        search.requires_admin,
        "the admin search needs the admin role"
    );
}

#[test]
fn the_me_preferences_and_consent_routes_are_identity_and_not_admin() {
    let table = RouteTable::default_registry_routes();

    for (method, path) in [
        (Method::Get, "/v1/me/preferences"),
        (Method::Put, "/v1/me/preferences"),
        (Method::Get, "/v1/me/consent/state"),
        (Method::Get, "/v1/me/consent"),
        (Method::Post, "/v1/me/consent"),
        (Method::Post, "/v1/me/consent/terms/withdraw"),
    ] {
        let route = table.match_route(method, path).expect("U4 route");
        assert_eq!(route.service.as_str(), "identity", "{method:?} {path}");
        assert!(!route.requires_admin, "{method:?} {path}");
    }

    assert!(
        table
            .match_route(Method::Get, "/v1/me/consent/terms")
            .is_none(),
        "only the declared sub-paths are served"
    );
}

#[test]
fn the_admin_role_change_route_is_declared_and_requires_admin() {
    let table = RouteTable::default_registry_routes();
    let route = table
        .match_route(
            Method::Put,
            "/v1/admin/users/11111111-1111-4111-8111-111111111111/role",
        )
        .expect("role-change route");
    assert_eq!(route.service.as_str(), "identity");
    assert!(
        route.requires_admin,
        "the role change needs the admin role at the edge"
    );
}

#[test]
fn roles_compare_by_privilege() {
    assert_eq!(Role::Admin, Role::Admin);
    assert_ne!(Role::Admin, Role::User);
    let route = Route {
        method: Method::Get,
        pattern: PathPattern::parse("/v1/assets/resolve").unwrap(),
        service: ServiceName::registry(),
        requires_admin: false,
    };
    assert_eq!(route.method, Method::Get);
}

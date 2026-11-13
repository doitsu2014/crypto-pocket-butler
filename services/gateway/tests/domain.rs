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
fn the_identity_routes_are_declared_with_the_owner_reads_open() {
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

    assert!(
        table.match_route(Method::Get, "/v1/admin/users").is_none(),
        "the address-carrying GET admin search is gone (D-S1)"
    );
}

#[test]
fn the_u10_admin_routes_are_declared_with_their_own_budgets() {
    let table = RouteTable::default_registry_routes();
    let user = "11111111-1111-4111-8111-111111111111";

    let cases = [
        // D-S1: the admin lookup carries its address in a body, never the URL.
        (Method::Post, "/v1/admin/users/lookup".to_string(), Some(60)),
        (
            Method::Put,
            format!("/v1/admin/users/{user}/role"),
            Some(20),
        ),
        (
            Method::Post,
            format!("/v1/admin/users/{user}/disable"),
            Some(20),
        ),
        (
            Method::Post,
            format!("/v1/admin/users/{user}/restore"),
            Some(20),
        ),
        (Method::Get, "/v1/support-grants".to_string(), Some(60)),
    ];

    for (method, path, budget) in cases {
        let route = table
            .match_route(method, &path)
            .unwrap_or_else(|| panic!("{method:?} {path} is declared"));
        assert_eq!(route.service.as_str(), "identity", "{method:?} {path}");
        assert!(
            route.requires_admin,
            "{method:?} {path} needs the admin role"
        );
        assert_eq!(
            route.max_requests_per_minute, budget,
            "{method:?} {path}: the U10 routes carry their own per-token budget"
        );
    }

    // The table stays deny-by-default around the new declarations.
    assert!(table
        .match_route(Method::Get, "/v1/admin/users/lookup")
        .is_none());
    assert!(table
        .match_route(Method::Put, "/v1/admin/users/lookup")
        .is_none());
    assert!(table.match_route(Method::Post, "/v1/admin/users").is_none());
    assert!(table
        .match_route(Method::Post, &format!("/v1/admin/users/{user}"))
        .is_none());
}

#[test]
fn the_role_change_budget_is_read_from_the_only_declared_entry() {
    let table = RouteTable::default_registry_routes();
    let path = "/v1/admin/users/11111111-1111-4111-8111-111111111111/role";

    // `RouteTable::match_route` takes the first match and the gateway reads the
    // budget from that same entry, so a second PUT entry would be shadowed and
    // its 20/min would never apply.
    let matching: Vec<_> = table
        .routes()
        .iter()
        .filter(|route| route.method == Method::Put && route.pattern.matches(path))
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "a duplicate PUT /v1/admin/users/{{userId}}/role entry would be shadowed"
    );
    assert_eq!(matching[0].max_requests_per_minute, Some(20));

    let matched = table.match_route(Method::Put, path).expect("role route");
    assert_eq!(matched.max_requests_per_minute, Some(20));
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
        max_requests_per_minute: None,
    };
    assert_eq!(route.method, Method::Get);
}

#[test]
fn the_support_routes_are_declared_with_their_per_route_budgets() {
    let table = RouteTable::default_registry_routes();
    let grant = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";

    let cases = [
        (
            Method::Post,
            "/v1/support-grants".to_string(),
            false,
            Some(20),
        ),
        (
            Method::Post,
            format!("/v1/support-grants/{grant}/approve"),
            true,
            None,
        ),
        (
            Method::Delete,
            format!("/v1/support-grants/{grant}"),
            true,
            None,
        ),
        (
            Method::Get,
            format!("/v1/support-grants/{grant}/metadata"),
            false,
            Some(60),
        ),
    ];

    for (method, path, requires_admin, budget) in cases {
        let route = table
            .match_route(method, &path)
            .unwrap_or_else(|| panic!("{method:?} {path} is declared"));
        assert_eq!(route.service.as_str(), "identity", "{method:?} {path}");
        assert_eq!(route.requires_admin, requires_admin, "{method:?} {path}");
        assert_eq!(
            route.max_requests_per_minute, budget,
            "{method:?} {path}: only the manual creation and the read carry a budget"
        );
    }
}

#[test]
fn the_mine_route_is_declared_without_a_role_or_a_private_budget() {
    let table = RouteTable::default_registry_routes();
    let path = "/v1/support-grants/mine";

    let route = table
        .match_route(Method::Get, path)
        .expect("the mine route is declared");
    assert_eq!(route.service.as_str(), "identity");
    assert!(
        !route.requires_admin,
        "a support agent's own read needs no admin role"
    );
    assert_eq!(
        route.max_requests_per_minute, None,
        "the mine read keeps the shared per-IP budget"
    );
    assert_eq!(
        route.pattern.as_str(),
        path,
        "the literal route answers, not a broader pattern"
    );

    // The table is first-match: exactly one declared GET pattern answers this
    // path, so no broader entry can shadow the mine read.
    let matching: Vec<_> = table
        .routes()
        .iter()
        .filter(|candidate| candidate.method == Method::Get && candidate.pattern.matches(path))
        .collect();
    assert_eq!(matching.len(), 1, "one GET route answers the mine path");

    // Its undeclared GET neighbours stay denied. (`DELETE .../mine` is matched
    // by the declared revoke pattern as an invalid grant id, which Identity
    // refuses; no GET pattern does.)
    assert!(table
        .match_route(
            Method::Get,
            "/v1/support-grants/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
        )
        .is_none());
}

#[test]
fn a_declared_route_without_a_budget_stays_unlimited() {
    let table = RouteTable::default_registry_routes();
    let route = table
        .match_route(
            Method::Get,
            "/v1/support-grants/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa/metadata",
        )
        .expect("metadata route");
    assert_eq!(route.max_requests_per_minute, Some(60));

    let approve = table
        .match_route(
            Method::Post,
            "/v1/support-grants/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa/approve",
        )
        .expect("approve route");
    assert_eq!(approve.max_requests_per_minute, None);
}

#[test]
fn the_settings_routes_are_declared_for_identity_with_the_deletion_budget() {
    let table = RouteTable::default_registry_routes();
    let export_id = "11111111-1111-4111-8111-111111111111";
    let cases = [
        (Method::Get, "/v1/me/account-status".to_string(), None),
        (Method::Get, "/v1/me/profile".to_string(), None),
        (Method::Patch, "/v1/me/profile".to_string(), None),
        (Method::Get, "/v1/me/deletion-request".to_string(), None),
        (Method::Delete, "/v1/me/deletion-request".to_string(), None),
        // D3 is the manual trigger, so it carries its own 5/min budget.
        (Method::Post, "/v1/privacy/deletions".to_string(), Some(5)),
        (Method::Post, "/v1/exports".to_string(), None),
        (Method::Get, format!("/v1/exports/{export_id}"), None),
    ];

    for (method, path, budget) in cases {
        let route = table
            .match_route(method, &path)
            .unwrap_or_else(|| panic!("{method:?} {path} is declared"));
        assert_eq!(route.service.as_str(), "identity", "{method:?} {path}");
        assert!(!route.requires_admin, "{method:?} {path}");
        assert_eq!(
            route.max_requests_per_minute, budget,
            "{method:?} {path}: only the manual deletion trigger carries a budget"
        );
    }

    // The table stays deny-by-default: only the declared methods and paths are
    // served, so the neighbours of the new routes are still unknown.
    assert!(table
        .match_route(Method::Get, "/v1/privacy/deletions")
        .is_none());
    assert!(table.match_route(Method::Post, "/v1/me/profile").is_none());
    assert!(table.match_route(Method::Get, "/v1/exports").is_none());
    assert!(table
        .match_route(Method::Delete, &format!("/v1/exports/{export_id}"))
        .is_none());
}

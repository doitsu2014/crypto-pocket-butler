//! The declared route table. Anything not declared is denied (BR4.2).
//!
//! The four support-access routes and their per-token budgets are declared here
//! (FR16.3–FR16.6, NFR11.1), as is the support agent's own-grant read (S2) with
//! no per-route budget, the eight settings routes and the manual deletion
//! trigger's 5/min budget (U7); a route without a budget keeps the address
//! limit.

use crate::error::GatewayError;
use crate::method::Method;
use crate::path::PathPattern;

/// The owning service of a route.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceName(String);

impl ServiceName {
    /// Creates a service name.
    ///
    /// # Errors
    /// Returns [`GatewayError::BadRequest`] for a blank value.
    pub fn new(raw: impl Into<String>) -> Result<Self, GatewayError> {
        let value = raw.into();
        if value.trim().is_empty() {
            return Err(GatewayError::BadRequest(
                "service name must not be blank".to_string(),
            ));
        }
        Ok(Self(value.trim().to_string()))
    }

    /// The Asset Registry service.
    #[must_use]
    pub fn registry() -> Self {
        Self("asset-registry".to_string())
    }

    /// The Identity service.
    #[must_use]
    pub fn identity() -> Self {
        Self("identity".to_string())
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One declared route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// The request method.
    pub method: Method,
    /// The declared path pattern.
    pub pattern: PathPattern,
    /// The owning service.
    pub service: ServiceName,
    /// Whether the admin role is required.
    pub requires_admin: bool,
    /// A per-route request budget in requests per minute (NFR11.1). `None`
    /// leaves the route on the shared per-IP budget alone.
    pub max_requests_per_minute: Option<u32>,
}

impl Route {
    /// The declared path pattern, used as the per-route budget's bucket label.
    #[must_use]
    pub fn pattern_label(&self) -> &str {
        self.pattern.as_str()
    }
}

/// The deny-by-default route table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteTable {
    routes: Vec<Route>,
}

impl RouteTable {
    /// Creates a table from explicit routes.
    #[must_use]
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    /// The declared service routes. Health, readiness, and metrics are handled
    /// by the gateway itself and are deliberately absent.
    ///
    /// The name keeps the walking-skeleton origin; the table now also declares
    /// the Identity routes (U1), so each owning service has its upstream.
    #[must_use]
    pub fn default_registry_routes() -> Self {
        let registry = ServiceName::registry();
        let identity = ServiceName::identity();
        let route_for = |service: &ServiceName, method, pattern: &str, requires_admin| Route {
            method,
            pattern: PathPattern::parse(pattern).expect("declared patterns are valid"),
            service: service.clone(),
            requires_admin,
            max_requests_per_minute: None,
        };
        let route = |method, pattern: &str, requires_admin| {
            route_for(&registry, method, pattern, requires_admin)
        };
        let identity_route = |method, pattern: &str, requires_admin| {
            route_for(&identity, method, pattern, requires_admin)
        };
        let limited_identity_route = |method, pattern: &str, requires_admin, limit: u32| Route {
            max_requests_per_minute: Some(limit),
            ..route_for(&identity, method, pattern, requires_admin)
        };
        Self::new(vec![
            route(Method::Get, "/v1/assets/resolve", false),
            route(Method::Get, "/v1/assets/{assetId}/price", false),
            route(Method::Post, "/v1/admin/assets", true),
            route(Method::Post, "/v1/admin/token-contracts", true),
            route(Method::Put, "/v1/admin/price-sources", true),
            route(
                Method::Patch,
                "/v1/admin/price-sources/{name}/enabled",
                true,
            ),
            route(Method::Post, "/v1/admin/observations/{id}/resolve", true),
            route(Method::Post, "/v1/admin/observations/{id}/ignore", true),
            // U1: principal resolution and the owner/admin record read.
            identity_route(Method::Get, "/v1/principals/{authUserId}", false),
            identity_route(Method::Get, "/v1/users/{userId}", false),
            // U10: the admin exact-email lookup carries the address in a POST
            // body, never the URL (D-S1), and carries its own 60/min budget.
            limited_identity_route(Method::Post, "/v1/admin/users/lookup", true, 60),
            // U2: the admin role change. The JWT is forwarded unchanged and
            // Identity re-checks the admin role from that token. The budget is
            // set on this, the only declared entry: the matcher takes the first
            // match and the budget is read from it.
            limited_identity_route(Method::Put, "/v1/admin/users/{userId}/role", true, 20),
            // U10: the admin disable and restore actions (U5's use cases) carry
            // their own 20/min budget each.
            limited_identity_route(Method::Post, "/v1/admin/users/{userId}/disable", true, 20),
            limited_identity_route(Method::Post, "/v1/admin/users/{userId}/restore", true, 20),
            // U4: the person's own preferences and consent surface. The user id
            // comes from the forwarded JWT, never the path, so none of these
            // require the admin role.
            identity_route(Method::Get, "/v1/me/preferences", false),
            identity_route(Method::Put, "/v1/me/preferences", false),
            identity_route(Method::Get, "/v1/me/consent/state", false),
            identity_route(Method::Get, "/v1/me/consent", false),
            identity_route(Method::Post, "/v1/me/consent", false),
            identity_route(Method::Post, "/v1/me/consent/{kind}/withdraw", false),
            // U3: support access. The grant creation and the metadata read carry
            // their own per-token budgets (NFR11.1); approval and revocation stay
            // on the shared per-IP budget and the admin role. U10 adds the
            // bounded pending-grant list an admin reviews, at 60/min.
            limited_identity_route(Method::Post, "/v1/support-grants", false, 20),
            limited_identity_route(Method::Get, "/v1/support-grants", true, 60),
            // S2: a support agent's own grants. Self-scoped through the
            // forwarded JWT, so no admin role and no per-route budget. It is
            // declared before the parameterised sibling and answers first.
            identity_route(Method::Get, "/v1/support-grants/mine", false),
            identity_route(Method::Post, "/v1/support-grants/{grantId}/approve", true),
            identity_route(Method::Delete, "/v1/support-grants/{grantId}", true),
            limited_identity_route(
                Method::Get,
                "/v1/support-grants/{grantId}/metadata",
                false,
                60,
            ),
            // U7: the settings surface. The account-status and profile reads, the
            // profile write, the open deletion-request read and the cancel are all
            // self-scoped through the forwarded JWT, so none needs the admin role.
            // The manual deletion trigger (D3) and the two export routes carry
            // their own per-token budgets; D3 is the strictest at 5/min.
            identity_route(Method::Get, "/v1/me/account-status", false),
            identity_route(Method::Get, "/v1/me/profile", false),
            identity_route(Method::Patch, "/v1/me/profile", false),
            identity_route(Method::Get, "/v1/me/deletion-request", false),
            identity_route(Method::Delete, "/v1/me/deletion-request", false),
            limited_identity_route(Method::Post, "/v1/privacy/deletions", false, 5),
            identity_route(Method::Post, "/v1/exports", false),
            identity_route(Method::Get, "/v1/exports/{exportId}", false),
        ])
    }

    /// Finds the declared route for a request, if any.
    #[must_use]
    pub fn match_route(&self, method: Method, path: &str) -> Option<&Route> {
        self.routes
            .iter()
            .find(|route| route.method == method && route.pattern.matches(path))
    }

    /// Every declared route.
    #[must_use]
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}

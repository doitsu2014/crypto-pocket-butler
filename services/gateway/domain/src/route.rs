//! The declared route table. Anything not declared is denied (BR4.2).

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
        };
        let route = |method, pattern: &str, requires_admin| {
            route_for(&registry, method, pattern, requires_admin)
        };
        let identity_route = |method, pattern: &str, requires_admin| {
            route_for(&identity, method, pattern, requires_admin)
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
            // U1: principal resolution, the owner/admin record read, and the
            // admin exact-email search.
            identity_route(Method::Get, "/v1/principals/{authUserId}", false),
            identity_route(Method::Get, "/v1/users/{userId}", false),
            identity_route(Method::Get, "/v1/admin/users", true),
            // U2: the admin role change. The JWT is forwarded unchanged and
            // Identity re-checks the admin role from that token.
            identity_route(Method::Put, "/v1/admin/users/{userId}/role", true),
            // U4: the person's own preferences and consent surface. The user id
            // comes from the forwarded JWT, never the path, so none of these
            // require the admin role.
            identity_route(Method::Get, "/v1/me/preferences", false),
            identity_route(Method::Put, "/v1/me/preferences", false),
            identity_route(Method::Get, "/v1/me/consent/state", false),
            identity_route(Method::Get, "/v1/me/consent", false),
            identity_route(Method::Post, "/v1/me/consent", false),
            identity_route(Method::Post, "/v1/me/consent/{kind}/withdraw", false),
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

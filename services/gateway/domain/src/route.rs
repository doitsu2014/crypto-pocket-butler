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

    /// The walking-skeleton registry routes. Health, readiness, and metrics are
    /// handled by the gateway itself and are deliberately absent.
    #[must_use]
    pub fn default_registry_routes() -> Self {
        let registry = ServiceName::registry();
        let route = |method, pattern: &str, requires_admin| Route {
            method,
            pattern: PathPattern::parse(pattern).expect("declared patterns are valid"),
            service: registry.clone(),
            requires_admin,
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

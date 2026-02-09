/// Keycloak configuration for JWT validation
#[derive(Debug, Clone)]
pub struct KeycloakConfig {
    /// Keycloak issuer URL (e.g., "https://keycloak.example.com/realms/myrealm")
    pub issuer: String,
    /// Expected audience in the JWT (e.g., "account" or your client_id)
    pub audience: String,
    /// JWKS endpoint URL (typically {issuer}/protocol/openid-connect/certs)
    pub jwks_url: String,
}

impl KeycloakConfig {
    /// Create a new Keycloak configuration
    pub fn new(issuer: String, audience: String) -> Self {
        let jwks_url = format!("{}/protocol/openid-connect/certs", issuer);
        Self {
            issuer,
            audience,
            jwks_url,
        }
    }

    /// Create a new Keycloak configuration with custom JWKS URL
    pub fn with_jwks_url(issuer: String, audience: String, jwks_url: String) -> Self {
        Self {
            issuer,
            audience,
            jwks_url,
        }
    }
}

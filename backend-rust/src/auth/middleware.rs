use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::sync::Arc;

use super::{jwt::JwtError, Claims, JwtValidator};

/// User information extracted from JWT for use in handlers
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User ID from JWT sub claim
    pub user_id: String,
    /// Preferred username (optional)
    pub username: Option<String>,
    /// Email address (optional)
    pub email: Option<String>,
    /// Full claims from JWT
    pub claims: Claims,
}

impl AuthUser {
    fn from_claims(claims: Claims) -> Self {
        Self {
            user_id: claims.sub.clone(),
            username: claims.preferred_username.clone(),
            email: claims.email.clone(),
            claims,
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for JwtError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            JwtError::MissingAuthHeader => (StatusCode::UNAUTHORIZED, "Missing authorization header"),
            JwtError::InvalidAuthHeader => (StatusCode::UNAUTHORIZED, "Invalid authorization header format"),
            JwtError::DecodeHeader(_) => (StatusCode::UNAUTHORIZED, "Invalid token format"),
            JwtError::ValidationError(_) => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
            JwtError::JwksFetchError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Authentication service unavailable"),
            JwtError::KeyNotFound => (StatusCode::UNAUTHORIZED, "Token signing key not found"),
            JwtError::InvalidIssuer => (StatusCode::UNAUTHORIZED, "Invalid token issuer"),
            JwtError::InvalidAudience => (StatusCode::UNAUTHORIZED, "Invalid token audience"),
        };

        let body = serde_json::to_string(&ErrorResponse {
            error: message.to_string(),
        })
        .unwrap();

        (status, body).into_response()
    }
}

/// Middleware that validates JWT and adds AuthUser to request extensions
pub async fn require_auth(
    State(validator): State<Arc<JwtValidator>>,
    mut request: Request,
    next: Next,
) -> Result<Response, JwtError> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(JwtError::MissingAuthHeader)?;

    // Extract token from header
    let token = JwtValidator::extract_token(auth_header)?;

    // Validate token
    let claims = validator.validate(token).await?;

    // Create AuthUser and add to request extensions
    let auth_user = AuthUser::from_claims(claims);
    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_from_claims() {
        let claims = Claims {
            sub: "user-456".to_string(),
            iss: "https://keycloak.example.com/realms/test".to_string(),
            aud: serde_json::Value::String("account".to_string()),
            exp: 1234567890,
            iat: 1234567800,
            preferred_username: Some("johndoe".to_string()),
            email: Some("john@example.com".to_string()),
            email_verified: Some(true),
        };

        let auth_user = AuthUser::from_claims(claims);
        assert_eq!(auth_user.user_id, "user-456");
        assert_eq!(auth_user.username, Some("johndoe".to_string()));
        assert_eq!(auth_user.email, Some("john@example.com".to_string()));
    }
}

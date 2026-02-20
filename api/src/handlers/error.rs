use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

/// Standard API error response format
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

/// Centralized API error enum for consistent error handling across all endpoints
#[derive(Debug)]
pub enum ApiError {
    /// 400 Bad Request - Invalid input or validation failure
    BadRequest(String),
    
    /// 401 Unauthorized - Missing or invalid authentication
    Unauthorized,
    
    /// 403 Forbidden - Authenticated but lacks permission
    Forbidden,
    
    /// 404 Not Found - Resource does not exist
    NotFound,
    
    /// 409 Conflict - Resource already exists or state conflict
    Conflict(String),
    
    /// 500 Internal Server Error - Database or other internal errors
    DatabaseError(sea_orm::DbErr),
    
    /// 500 Internal Server Error - Generic server error
    InternalServerError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::DatabaseError(err) => {
                // Log the actual database error for debugging
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ApiError::InternalServerError(msg) => {
                // Log the internal error for debugging
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

// Convenient conversion from sea_orm database errors
impl From<sea_orm::DbErr> for ApiError {
    fn from(err: sea_orm::DbErr) -> Self {
        ApiError::DatabaseError(err)
    }
}

// Convenient conversion from standard errors
impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::InternalServerError(format!("IO error: {}", err))
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Forbidden => write!(f, "Forbidden"),
            ApiError::NotFound => write!(f, "Not Found"),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::DatabaseError(err) => write!(f, "Database Error: {:?}", err),
            ApiError::InternalServerError(msg) => write!(f, "Internal Server Error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

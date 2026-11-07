//! RFC 7807 `application/problem+json` responses carrying the correlation id and,
//! for a validation refusal, the offending field paths only.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// The problem content type.
pub const PROBLEM_CONTENT_TYPE: &str = "application/problem+json";

/// One offending field path, never its value.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldProblem {
    /// The field path.
    pub field: String,
    /// Why it was refused.
    pub reason: String,
}

/// The RFC 7807 problem body.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemDetails {
    /// A stable problem-type URI.
    pub r#type: String,
    /// A short, human-readable summary.
    pub title: String,
    /// The HTTP status code.
    pub status: u16,
    /// A human-readable explanation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The offending field paths, on a validation refusal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldProblem>>,
    /// The request correlation id.
    pub correlation_id: String,
}

impl ProblemDetails {
    /// Creates a problem.
    #[must_use]
    pub fn new(
        status: u16,
        problem_type: &str,
        title: &str,
        detail: Option<String>,
        correlation_id: &str,
    ) -> Self {
        Self {
            r#type: format!("https://errors.cpb/analytics/{problem_type}"),
            title: title.to_string(),
            status,
            detail,
            errors: None,
            correlation_id: correlation_id.to_string(),
        }
    }

    /// Adds field-level detail.
    #[must_use]
    pub fn with_fields(mut self, fields: Vec<FieldProblem>) -> Self {
        self.errors = Some(fields);
        self
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response = (status, Json(self)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(PROBLEM_CONTENT_TYPE),
        );
        response
    }
}

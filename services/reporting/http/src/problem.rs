//! RFC 7807 `application/problem+json` responses carrying the correlation id and,
//! for a validation refusal, the offending field paths only.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use reporting_application::common::{AppError, FieldError};
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
    /// A human-readable explanation, when one helps.
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
            r#type: format!("https://errors.cpb/reporting/{problem_type}"),
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

    /// A 500 problem for an unexpected failure.
    #[must_use]
    pub fn internal(correlation_id: &str) -> Self {
        Self::new(
            500,
            "internal",
            "Internal server error",
            None,
            correlation_id,
        )
    }

    /// A 404 problem.
    #[must_use]
    pub fn not_found(correlation_id: &str, detail: impl Into<String>) -> Self {
        Self::new(
            404,
            "not-found",
            "Not found",
            Some(detail.into()),
            correlation_id,
        )
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

/// Maps an application refusal to the RFC 7807 response.
#[must_use]
pub fn from_app_error(error: &AppError, correlation_id: &str) -> ProblemDetails {
    match error {
        AppError::Unauthorized(detail) => ProblemDetails::new(
            401,
            "unauthorized",
            "Unauthorized",
            Some(detail.clone()),
            correlation_id,
        ),
        AppError::Forbidden(detail) => ProblemDetails::new(
            403,
            "forbidden",
            "Forbidden",
            Some(detail.clone()),
            correlation_id,
        ),
        AppError::NotFound(what) => {
            ProblemDetails::not_found(correlation_id, format!("{what} was not found"))
        }
        AppError::Validation(errors) => ProblemDetails::new(
            422,
            "validation",
            "Invalid request",
            Some("one or more fields failed validation".to_string()),
            correlation_id,
        )
        .with_fields(
            errors
                .iter()
                .map(|error| FieldProblem {
                    field: error.field.clone(),
                    reason: error.reason.clone(),
                })
                .collect(),
        ),
        AppError::Conflict(detail) => ProblemDetails::new(
            409,
            "conflict",
            "Conflict",
            Some(detail.clone()),
            correlation_id,
        ),
        AppError::Unavailable(detail) => ProblemDetails::new(
            503,
            "unavailable",
            "Service unavailable",
            Some(detail.clone()),
            correlation_id,
        ),
        AppError::Internal(detail) => {
            tracing::error!(%detail, "reporting request failed internally");
            ProblemDetails::internal(correlation_id)
        }
    }
}

/// Builds a validation problem from field errors.
#[must_use]
pub fn validation(correlation_id: &str, errors: Vec<FieldError>) -> ProblemDetails {
    from_app_error(&AppError::Validation(errors), correlation_id)
}

#[cfg(test)]
mod tests {
    use super::{from_app_error, ProblemDetails};
    use reporting_application::common::{AppError, FieldError};

    fn status_of(error: &AppError) -> u16 {
        from_app_error(error, "c-1").status
    }

    #[test]
    fn every_application_refusal_maps_to_its_status() {
        assert_eq!(status_of(&AppError::Unauthorized("no".into())), 401);
        assert_eq!(status_of(&AppError::Forbidden("no".into())), 403);
        assert_eq!(status_of(&AppError::NotFound("brief")), 404);
        assert_eq!(status_of(&AppError::Conflict("no".into())), 409);
        assert_eq!(
            status_of(&AppError::Validation(vec![FieldError::new(
                "limit", "too big"
            )])),
            422
        );
        assert_eq!(status_of(&AppError::Unavailable("down".into())), 503);
        assert_eq!(status_of(&AppError::Internal("bug".into())), 500);
    }

    #[test]
    fn a_validation_refusal_names_the_field_paths_only() {
        let problem = from_app_error(
            &AppError::Validation(vec![FieldError::new("limit", "must be between 1 and 100")]),
            "c-1",
        );
        let errors = problem.errors.expect("the field paths are present");
        assert_eq!(errors[0].field, "limit");
    }

    #[test]
    fn a_problem_carries_the_correlation_id() {
        let problem = ProblemDetails::internal("c-9");
        assert_eq!(problem.correlation_id, "c-9");
        assert_eq!(problem.status, 500);
    }

    #[test]
    fn a_not_found_problem_names_what_is_missing() {
        let problem = ProblemDetails::not_found("c-2", "brief was not found");
        assert_eq!(problem.status, 404);
        assert_eq!(problem.detail.as_deref(), Some("brief was not found"));
    }
}

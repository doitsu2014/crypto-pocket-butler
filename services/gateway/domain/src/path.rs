//! Declared path patterns. A pattern is a sequence of literal and parameter
//! segments; a parameter matches exactly one non-empty segment.

use crate::error::GatewayError;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Parameter,
}

/// A path pattern such as `/v1/assets/{assetId}/price`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPattern {
    segments: Vec<Segment>,
}

impl PathPattern {
    /// Parses a pattern.
    ///
    /// # Errors
    /// Returns [`GatewayError::BadRequest`] when the pattern does not start with
    /// `/`, contains an empty segment, or declares an empty parameter name.
    pub fn parse(raw: &str) -> Result<Self, GatewayError> {
        let trimmed = raw.trim();
        let Some(rest) = trimmed.strip_prefix('/') else {
            return Err(GatewayError::BadRequest(format!(
                "path pattern must start with '/': {raw}"
            )));
        };
        if rest.is_empty() {
            return Ok(Self {
                segments: Vec::new(),
            });
        }
        let mut segments = Vec::new();
        for raw_segment in rest.split('/') {
            if raw_segment.is_empty() {
                return Err(GatewayError::BadRequest(format!(
                    "path pattern contains an empty segment: {raw}"
                )));
            }
            if let Some(parameter) = raw_segment
                .strip_prefix('{')
                .and_then(|value| value.strip_suffix('}'))
            {
                if parameter.is_empty() {
                    return Err(GatewayError::BadRequest(format!(
                        "path pattern contains an unnamed parameter: {raw}"
                    )));
                }
                segments.push(Segment::Parameter);
            } else {
                segments.push(Segment::Literal(raw_segment.to_string()));
            }
        }
        Ok(Self { segments })
    }

    /// Whether a concrete request path matches.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        let path = path.trim();
        let Some(rest) = path.strip_prefix('/') else {
            return false;
        };
        let concrete: Vec<&str> = if rest.is_empty() {
            Vec::new()
        } else {
            rest.split('/').collect()
        };
        if concrete.len() != self.segments.len() {
            return false;
        }
        self.segments
            .iter()
            .zip(concrete)
            .all(|(segment, actual)| match segment {
                Segment::Literal(expected) => expected == actual,
                Segment::Parameter => !actual.is_empty(),
            })
    }
}

//! Domain errors. A refusal names the rule and the field path, never the
//! offending value.

/// A business-rule violation, expressed without transport or storage detail.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A required field was absent or blank.
    #[error("{field} is required")]
    Required {
        /// The field path that failed.
        field: &'static str,
    },

    /// A field carried a value outside its allowed shape.
    #[error("{field}: {reason}")]
    Invalid {
        /// The field path that failed.
        field: &'static str,
        /// Why the value was refused, phrased without the value itself.
        reason: String,
    },

    /// A bounded field exceeded its limit.
    #[error("{field} exceeds {limit}")]
    TooLarge {
        /// The field path that failed.
        field: &'static str,
        /// A human-readable bound description.
        limit: &'static str,
    },

    /// A state-machine transition was not allowed.
    #[error("{entity} cannot move from {from} to {to}")]
    InvalidTransition {
        /// The entity name.
        entity: &'static str,
        /// The current state label.
        from: &'static str,
        /// The requested state label.
        to: &'static str,
    },

    /// An identifier was not a well-formed UUID.
    #[error("{entity} identifier is not a UUID")]
    InvalidId {
        /// The entity whose identifier failed.
        entity: &'static str,
    },

    /// A rule refused the action.
    #[error("{reason}")]
    Denied {
        /// The plain reason, safe to show the caller.
        reason: String,
    },

    /// The action conflicts with the record's current state.
    #[error("{reason}")]
    Conflict {
        /// The plain reason, safe to show the caller.
        reason: String,
    },
}

impl DomainError {
    /// The field path the refusal concerns, suitable for a problem document.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            Self::Required { field }
            | Self::Invalid { field, .. }
            | Self::TooLarge { field, .. } => field,
            Self::InvalidTransition { .. } => "state",
            Self::InvalidId { entity } => entity,
            Self::Denied { .. } | Self::Conflict { .. } => "request",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DomainError;

    #[test]
    fn every_variant_names_a_field_path() {
        assert_eq!(DomainError::Required { field: "period" }.field(), "period");
        assert_eq!(
            DomainError::Invalid {
                field: "asOf",
                reason: "is in the future".into()
            }
            .field(),
            "asOf"
        );
        assert_eq!(
            DomainError::TooLarge {
                field: "failureReason",
                limit: "500 characters"
            }
            .field(),
            "failureReason"
        );
        assert_eq!(
            DomainError::InvalidTransition {
                entity: "brief",
                from: "failed",
                to: "pending"
            }
            .field(),
            "state"
        );
        assert_eq!(DomainError::InvalidId { entity: "brief" }.field(), "brief");
        assert_eq!(
            DomainError::Denied {
                reason: "not yours".into()
            }
            .field(),
            "request"
        );
        assert_eq!(
            DomainError::Conflict {
                reason: "already generated".into()
            }
            .field(),
            "request"
        );
    }
}

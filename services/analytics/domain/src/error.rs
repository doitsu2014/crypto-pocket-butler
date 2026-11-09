//! Domain errors. A refusal names the field path and never repeats the value:
//! BR2.2 refuses a PII-shaped event name without echoing it.

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

    /// BR2.2: the value looked like personal data or a credential.
    #[error("{field} carries a personal-data or credential-shaped value")]
    PiiShaped {
        /// The field path that failed; the value is never repeated.
        field: &'static str,
    },

    /// A bounded field exceeded its limit.
    #[error("{field} exceeds {limit}")]
    TooLarge {
        /// The field path that failed.
        field: &'static str,
        /// A human-readable bound description.
        limit: &'static str,
    },

    /// BR2.1: the event name is not in the closed catalogue.
    #[error("eventName is not in the closed event catalogue")]
    UnknownEvent,

    /// An identifier was not a well-formed UUID.
    #[error("{entity} identifier is not a UUID")]
    InvalidId {
        /// The entity whose identifier failed.
        entity: &'static str,
    },
}

impl DomainError {
    /// The field path the refusal concerns.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            Self::Required { field }
            | Self::Invalid { field, .. }
            | Self::PiiShaped { field }
            | Self::TooLarge { field, .. } => field,
            Self::UnknownEvent => "eventName",
            Self::InvalidId { entity } => entity,
        }
    }
}

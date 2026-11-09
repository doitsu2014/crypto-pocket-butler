//! Domain errors. A refusal names the field path and never repeats the value.

/// A business-rule violation, expressed without transport or storage detail.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A required field was absent or blank.
    #[error("{field} is required")]
    Required {
        /// The field path that failed.
        field: String,
    },

    /// A field carried a value outside its allowed shape.
    #[error("{field}: {reason}")]
    Invalid {
        /// The field path that failed.
        field: String,
        /// Why the value was refused, phrased without the value itself.
        reason: String,
    },

    /// A bounded field exceeded its limit.
    #[error("{field} exceeds {limit}")]
    TooLarge {
        /// The field path that failed.
        field: String,
        /// A human-readable bound description.
        limit: String,
    },

    /// BR5.1: the value looked like a credential or a full address.
    #[error("{field} carries a credential or full address")]
    Sensitive {
        /// The field path that failed; the value is never repeated.
        field: String,
    },

    /// BR2.1 and BR3.1: a mandatory email preference cannot be turned off.
    #[error("{field} cannot be disabled")]
    Mandatory {
        /// The field path, e.g. `security.email`.
        field: String,
    },

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
    pub fn field(&self) -> &str {
        match self {
            Self::Required { field }
            | Self::Invalid { field, .. }
            | Self::TooLarge { field, .. }
            | Self::Sensitive { field }
            | Self::Mandatory { field } => field,
            Self::InvalidId { entity } => entity,
        }
    }
}

//! Domain errors. Every refusal names the rule and the field path it concerns,
//! never the offending value (BR1.3 rejects a secret without echoing it).

/// A business-rule violation, expressed without any transport or storage detail.
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

    /// BR1.3: a free-text field held a credential, token, seed phrase, or full address.
    #[error("{field} carries a credential-shaped value")]
    SecretShaped {
        /// The field path that failed; the value is never repeated.
        field: &'static str,
    },

    /// BR1.4: context carried a key outside the identifier allow-list.
    #[error("context.{key} is not an allowed context key")]
    ContextKeyNotAllowed {
        /// The rejected key, without the `context.` prefix.
        key: String,
    },

    /// A bounded field exceeded its limit.
    #[error("{field} exceeds {limit}")]
    TooLarge {
        /// The field path that failed.
        field: &'static str,
        /// A human-readable bound description.
        limit: &'static str,
    },

    /// Two supplied fields contradict each other.
    #[error("{field}: {reason}")]
    Contradiction {
        /// The field path whose value is refused.
        field: &'static str,
        /// Why the combination is refused.
        reason: String,
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
}

impl DomainError {
    /// The field path the refusal concerns, suitable for a problem document.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            Self::Required { field }
            | Self::Invalid { field, .. }
            | Self::SecretShaped { field }
            | Self::TooLarge { field, .. }
            | Self::Contradiction { field, .. } => field,
            Self::ContextKeyNotAllowed { .. } => "context",
            Self::InvalidTransition { .. } => "state",
            Self::InvalidId { entity } => entity,
        }
    }
}

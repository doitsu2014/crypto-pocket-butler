//! Typed identifiers. Each entity gets its own newtype so an observation id
//! cannot be passed where a refresh-run id is expected.

use uuid::Uuid;

use crate::error::DomainError;

macro_rules! uuid_id {
    ($name:ident, $entity:literal) => {
        #[doc = concat!("Identifier for a ", $entity, ".")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            /// Wraps an existing UUID.
            #[must_use]
            pub fn new(value: Uuid) -> Self {
                Self(value)
            }

            /// Generates a random identifier.
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }

            /// The underlying UUID.
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Parses a UUID string.
            ///
            /// # Errors
            /// Returns [`DomainError::InvalidId`] when the text is not a UUID.
            pub fn parse(raw: &str) -> Result<Self, DomainError> {
                Uuid::parse_str(raw.trim())
                    .map(Self)
                    .map_err(|_| DomainError::InvalidId { entity: $entity })
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

uuid_id!(ObservationId, "price observation");
uuid_id!(FxRateId, "fx rate");
uuid_id!(RefreshRunId, "refresh run");

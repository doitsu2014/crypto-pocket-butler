//! Typed identifiers. Each entity gets its own newtype so a brief id cannot be
//! passed where a user id is expected.

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

uuid_id!(UserId, "user");
uuid_id!(BriefId, "brief");
uuid_id!(BriefDeliveryId, "brief delivery");

#[cfg(test)]
mod tests {
    use super::{BriefId, UserId};
    use crate::error::DomainError;

    #[test]
    fn a_generated_id_round_trips_through_text() {
        let id = BriefId::generate();
        assert_eq!(BriefId::parse(&id.to_string()), Ok(id));
    }

    #[test]
    fn parsing_refuses_text_that_is_not_a_uuid() {
        assert_eq!(
            UserId::parse("not-a-uuid"),
            Err(DomainError::InvalidId { entity: "user" })
        );
    }

    #[test]
    fn parsing_trims_surrounding_whitespace() {
        let id = UserId::generate();
        let padded = format!("  {id}  ");
        assert_eq!(UserId::parse(&padded), Ok(id));
    }

    #[test]
    fn distinct_ids_compare_unequal() {
        let first = BriefId::generate();
        let second = BriefId::generate();
        assert_ne!(first, second);
    }
}

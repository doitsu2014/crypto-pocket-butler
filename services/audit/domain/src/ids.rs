//! Typed UUID identifiers. Keeping each entity's identifier in its own type stops
//! a retention-run id being passed where an event id is expected.

use std::fmt;

use uuid::Uuid;

use crate::error::DomainError;

macro_rules! uuid_id {
    ($name:ident, $doc:literal, $entity:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            /// Wraps an existing UUID.
            #[must_use]
            pub fn new(id: Uuid) -> Self {
                Self(id)
            }

            /// Generates a fresh random identifier.
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }

            /// Parses a UUID string.
            ///
            /// # Errors
            /// Returns [`DomainError::InvalidId`] when the value is not a UUID.
            pub fn parse(raw: &str) -> Result<Self, DomainError> {
                Uuid::parse_str(raw.trim())
                    .map(Self)
                    .map_err(|_| DomainError::InvalidId { entity: $entity })
            }

            /// The underlying UUID.
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = DomainError;

            fn from_str(raw: &str) -> Result<Self, Self::Err> {
                Self::parse(raw)
            }
        }
    };
}

uuid_id!(
    AuditEventId,
    "The identifier of one audit event.",
    "audit event"
);
uuid_id!(
    SupportSessionId,
    "The identifier of one support access session.",
    "support session"
);
uuid_id!(
    RetentionRunId,
    "The identifier of one retention run.",
    "retention run"
);
uuid_id!(
    AuditExportId,
    "The identifier of one audit export request.",
    "audit export"
);
uuid_id!(
    DeletionRequestId,
    "Identity's account-deletion request identifier.",
    "deletion request"
);
uuid_id!(
    PrivacyDeletionId,
    "The identifier of one recorded privacy-deletion request.",
    "privacy deletion"
);
uuid_id!(UserId, "A user identifier owned by Identity.", "user");

//! Typed UUID identifiers. Each entity gets its own type so a consent-record id
//! cannot be passed where an export id is expected.

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
    UserId,
    "The internal user identifier every service keys on.",
    "user"
);
uuid_id!(
    ConsentRecordId,
    "The identifier of one recorded consent decision.",
    "consent record"
);
uuid_id!(
    SupportGrantId,
    "The identifier of one support access grant.",
    "support grant"
);
uuid_id!(
    ExportRequestId,
    "The identifier of one data-export request.",
    "export request"
);
uuid_id!(
    DeletionRequestId,
    "The identifier of one account-deletion request.",
    "deletion request"
);
uuid_id!(
    ShreddingKeyId,
    "The identifier of one per-subject crypto-shredding key record.",
    "shredding key"
);
uuid_id!(
    DeviceSessionId,
    "The identifier of one device-session row.",
    "device session"
);

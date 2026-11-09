//! Typed identifiers. Each entity gets its own newtype so an account id cannot
//! be passed where a sync-run id is expected.

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
uuid_id!(AccountId, "account");
uuid_id!(CredentialId, "credential");
uuid_id!(WalletConnectionId, "wallet connection");
uuid_id!(SyncRunId, "sync run");
uuid_id!(HoldingSnapshotId, "holding snapshot");
uuid_id!(ImportBatchId, "import batch");
uuid_id!(ImportedTransactionId, "imported transaction");
uuid_id!(ImportRowErrorId, "import row error");

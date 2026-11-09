//! Stable, opaque identifiers. The domain never assumes an ID format beyond
//! "non-empty, bounded, no whitespace or control characters".

use std::fmt;

use crate::error::DomainError;

const MAX_ID_LEN: usize = 128;

fn validate_id(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::InvalidId("identifier is empty".to_string()));
    }
    if value.len() > MAX_ID_LEN {
        return Err(DomainError::InvalidId(format!(
            "identifier is longer than {MAX_ID_LEN} characters"
        )));
    }
    if value.chars().any(char::is_whitespace) || value.chars().any(char::is_control) {
        return Err(DomainError::InvalidId(
            "identifier must not contain whitespace or control characters".to_string(),
        ));
    }
    Ok(value.to_string())
}

macro_rules! string_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Parses and validates an identifier.
            ///
            /// # Errors
            /// Returns [`DomainError::InvalidId`] when the value is blank, too long,
            /// or contains whitespace or control characters.
            pub fn new(raw: impl Into<String>) -> Result<Self, DomainError> {
                Ok(Self(validate_id(&raw.into())?))
            }

            /// The identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id!(AssetId, "Canonical asset identifier.");
string_id!(ChainId, "Chain identifier.");
string_id!(TokenContractId, "Token contract identifier.");
string_id!(PriceSourceId, "Price source identifier.");
string_id!(ObservationId, "Unmapped observation identifier.");

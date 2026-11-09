//! Validated user preferences (BR3.3): locale, base currency, and timezone.

use crate::error::DomainError;

/// The supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    /// English.
    #[default]
    En,
    /// Vietnamese.
    Vi,
}

impl Locale {
    /// Parses and validates a locale.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything but `en` or `vi`.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "en" => Ok(Self::En),
            "vi" => Ok(Self::Vi),
            _ => Err(DomainError::Invalid {
                field: "locale",
                reason: "must be en or vi".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Vi => "vi",
        }
    }
}

/// An upper-case ISO 4217 currency code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BaseCurrency(String);

impl BaseCurrency {
    /// Parses and validates the three-letter ISO 4217 lexical form.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] unless the value is three upper-case
    /// ASCII letters.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let value = raw.trim();
        let valid = value.len() == 3
            && value
                .chars()
                .all(|character| character.is_ascii_uppercase());
        if !valid {
            return Err(DomainError::Invalid {
                field: "baseCurrency",
                reason: "must be a three-letter upper-case ISO 4217 code".to_string(),
            });
        }
        Ok(Self(value.to_string()))
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for BaseCurrency {
    fn default() -> Self {
        Self("USD".to_string())
    }
}

/// An IANA timezone name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimeZone(String);

impl TimeZone {
    /// Parses and validates an IANA timezone name.
    ///
    /// The rule is the IANA lexical shape: `UTC`, or one or more `/`-separated
    /// non-empty segments built from ASCII letters, digits, `_`, `-`, `+`, and
    /// `.`. The database keeps the canonical IANA registry; Identity stores and
    /// returns the name without transforming it (NFR16.1).
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the value is not an IANA-shaped name.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let value = raw.trim();
        let invalid = || DomainError::Invalid {
            field: "timezone",
            reason: "must be a valid IANA timezone name".to_string(),
        };
        if value.is_empty() || value.len() > 64 {
            return Err(invalid());
        }
        if value == "UTC" {
            return Ok(Self(value.to_string()));
        }
        if !value.contains('/') || value.starts_with('/') || value.ends_with('/') {
            return Err(invalid());
        }
        for segment in value.split('/') {
            if segment.is_empty()
                || !segment.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '+' | '.')
                })
            {
                return Err(invalid());
            }
        }
        Ok(Self(value.to_string()))
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TimeZone {
    fn default() -> Self {
        Self("UTC".to_string())
    }
}

/// The preferences Identity stores and returns without transformation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preferences {
    /// The UI language.
    pub locale: Locale,
    /// The valuation base currency.
    pub base_currency: BaseCurrency,
    /// The rendering timezone.
    pub timezone: TimeZone,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            locale: Locale::En,
            base_currency: BaseCurrency::default(),
            timezone: TimeZone::default(),
        }
    }
}

//! The pre-sign-up jurisdiction and age gate (BR7.1–BR7.3, WF6).
//!
//! A country with no rule is blocked, never allowed. The gate collects no
//! personal data: only the declared country, age, and disclosure acknowledgement.

use chrono::{DateTime, Utc};

use crate::error::DomainError;

/// The lifetime of the signed gate token.
pub const GATE_TOKEN_TTL_MINUTES: i64 = 10;

/// An ISO 3166-1 alpha-2 country code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CountryCode(String);

impl CountryCode {
    /// Parses and validates the two-letter upper-case form.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] unless the value is two upper-case
    /// ASCII letters.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let value = raw.trim();
        let valid = value.len() == 2
            && value
                .chars()
                .all(|character| character.is_ascii_uppercase());
        if !valid {
            return Err(DomainError::Invalid {
                field: "jurisdiction",
                reason: "must be an ISO 3166-1 alpha-2 country code".to_string(),
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

/// A launch-market rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JurisdictionRule {
    /// The ISO 3166-1 alpha-2 country code.
    pub code: CountryCode,
    /// Whether sign-up is allowed.
    pub allowed: bool,
    /// The declared minimum age.
    pub minimum_age: u32,
    /// Whether the launch disclosures must be acknowledged.
    pub disclosures_required: bool,
    /// When the rule last changed.
    pub updated_at: DateTime<Utc>,
}

/// The gate's answer before any personal data is collected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// The declaration may proceed and a gate token is issued.
    Allowed,
    /// The launch disclosures must be acknowledged first.
    DisclosuresRequired,
    /// The declaration is refused with a plain explanation.
    Denied {
        /// The explanation, safe to show the visitor.
        reason: String,
    },
}

/// One pre-sign-up declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignupDeclaration {
    /// The declared country.
    pub jurisdiction: CountryCode,
    /// The declared age.
    pub declared_age: u32,
    /// Whether the launch disclosures were acknowledged.
    pub disclosures_acknowledged: bool,
}

impl SignupDeclaration {
    /// Validates a declaration's shape; the rule check is [`evaluate`].
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for a non-country code or an impossible
    /// declared age.
    pub fn parse(
        jurisdiction: &str,
        declared_age: u32,
        disclosures_acknowledged: bool,
    ) -> Result<Self, DomainError> {
        if declared_age == 0 || declared_age > 130 {
            return Err(DomainError::Invalid {
                field: "declaredAge",
                reason: "must be between 1 and 130".to_string(),
            });
        }
        Ok(Self {
            jurisdiction: CountryCode::parse(jurisdiction)?,
            declared_age,
            disclosures_acknowledged,
        })
    }
}

/// Applies the jurisdiction rule to a declaration (WF6 steps 3–4).
#[must_use]
pub fn evaluate(
    rule: Option<&JurisdictionRule>,
    declared_age: u32,
    disclosures_acknowledged: bool,
) -> GateOutcome {
    let Some(rule) = rule else {
        return GateOutcome::Denied {
            reason: "sign-up is not available from this jurisdiction yet".to_string(),
        };
    };
    if !rule.allowed {
        return GateOutcome::Denied {
            reason: "sign-up is not available from this jurisdiction".to_string(),
        };
    }
    if declared_age < rule.minimum_age {
        return GateOutcome::Denied {
            reason: format!(
                "you must be at least {} to create an account",
                rule.minimum_age
            ),
        };
    }
    if rule.disclosures_required && !disclosures_acknowledged {
        return GateOutcome::DisclosuresRequired;
    }
    GateOutcome::Allowed
}

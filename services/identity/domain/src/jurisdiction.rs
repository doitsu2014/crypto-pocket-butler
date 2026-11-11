//! Jurisdiction rules and decisions.
//!
//! Two related concerns live here, both fail-closed:
//!
//! - The pre-sign-up jurisdiction and age gate (BR7.1–BR7.3, WF6). A country
//!   with no rule is blocked, never allowed. The gate collects no personal data:
//!   only the declared country, age, and disclosure acknowledgement.
//! - The runtime jurisdiction list and its check decision (BR1.1–BR1.6, BR1.10,
//!   US6.3/US6.4). A country the active list does not name is restricted, so a
//!   partial or missing list can never allow a visitor.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

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

// ---------------------------------------------------------------------------
// The runtime jurisdiction list and the check decision (BR1.1–BR1.4, BR1.10)
// ---------------------------------------------------------------------------

/// The longest list-version label.
pub const MAX_LIST_VERSION_LEN: usize = 64;
/// The longest `source` label on one list entry.
pub const MAX_LIST_SOURCE_LEN: usize = 120;
/// The number of bytes in the SHA-256 list checksum.
pub const LIST_CHECKSUM_LEN: usize = 64;

/// One country's availability in a published list version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JurisdictionStatus {
    /// The country may proceed through the gate.
    Allowed,
    /// The country is refused.
    Restricted,
}

impl JurisdictionStatus {
    /// Parses the stored value. The two values are exact; casing is not ignored.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "allowed" => Ok(Self::Allowed),
            "restricted" => Ok(Self::Restricted),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be allowed or restricted".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Restricted => "restricted",
        }
    }
}

/// The signal(s) that made a check restricted (BR1.1–BR1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RestrictionSource {
    /// The self-declared country is restricted.
    SelfDeclared,
    /// The IP-resolved country is restricted.
    Ip,
    /// Both signals are restricted; the stricter signal wins.
    Both,
}

impl RestrictionSource {
    /// The wire value: `self_declared`, `ip`, or `both`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SelfDeclared => "self_declared",
            Self::Ip => "ip",
            Self::Both => "both",
        }
    }
}

/// The gate's answer: allowed, or restricted by one or both signals, citing the
/// list version it used (BR1.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    allowed: bool,
    restricted_by: Option<RestrictionSource>,
    list_version: String,
}

impl Decision {
    /// Whether the check may proceed.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// The restriction source, or `None` when the check is allowed.
    #[must_use]
    pub fn restricted_by(&self) -> Option<RestrictionSource> {
        self.restricted_by
    }

    /// The list version the decision cites.
    #[must_use]
    pub fn list_version(&self) -> &str {
        &self.list_version
    }
}

/// Evaluates both signals against the current list (BR1.1).
///
/// A restricted signal denies; when both restrict, the decision names both.
#[must_use]
pub fn decide(
    self_declared: JurisdictionStatus,
    ip: JurisdictionStatus,
    list_version: &str,
) -> Decision {
    let restricted_by = match (self_declared, ip) {
        (JurisdictionStatus::Restricted, JurisdictionStatus::Restricted) => {
            Some(RestrictionSource::Both)
        }
        (JurisdictionStatus::Restricted, JurisdictionStatus::Allowed) => {
            Some(RestrictionSource::SelfDeclared)
        }
        (JurisdictionStatus::Allowed, JurisdictionStatus::Restricted) => {
            Some(RestrictionSource::Ip)
        }
        (JurisdictionStatus::Allowed, JurisdictionStatus::Allowed) => None,
    };
    Decision {
        allowed: restricted_by.is_none(),
        restricted_by,
        list_version: list_version.to_string(),
    }
}

/// One country entry in a list version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JurisdictionListEntry {
    /// The ISO 3166-1 alpha-2 country code.
    pub code: CountryCode,
    /// The country's availability.
    pub status: JurisdictionStatus,
    /// Who maintains the entry, for example `compliance`.
    pub source: String,
    /// When the entry last changed.
    pub updated_at: DateTime<Utc>,
}

/// An immutable snapshot of the jurisdiction list (entities.md).
///
/// Every decision cites one version, so a published version is never edited: a
/// change publishes a new one and leaves the previous readable (BR1.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JurisdictionListVersion {
    version: String,
    updated_at: DateTime<Utc>,
    entries: Vec<JurisdictionListEntry>,
    checksum: String,
}

impl JurisdictionListVersion {
    /// Builds a version from its entries, sorting them by country code and
    /// computing the checksum over the ordered entries.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for a malformed version label, a blank or
    /// oversized source, or an empty or duplicate entry set.
    pub fn new(
        version: impl Into<String>,
        updated_at: DateTime<Utc>,
        entries: Vec<JurisdictionListEntry>,
    ) -> Result<Self, DomainError> {
        let version = version.into();
        validate_version(&version)?;

        let mut entries = entries;
        entries.sort_by(|left, right| left.code.cmp(&right.code));
        validate_entries(&entries)?;

        let checksum = checksum_of(&entries);
        Ok(Self {
            version,
            updated_at,
            entries,
            checksum,
        })
    }

    /// Rebuilds a version read from storage and refuses one whose stored
    /// checksum does not match its entries (BR1.6).
    ///
    /// # Errors
    /// Returns the same errors as [`Self::new`], and [`DomainError::Conflict`]
    /// when `checksum` does not match the digest of `entries`.
    pub fn from_stored(
        version: impl Into<String>,
        updated_at: DateTime<Utc>,
        checksum: &str,
        entries: Vec<JurisdictionListEntry>,
    ) -> Result<Self, DomainError> {
        let list = Self::new(version, updated_at, entries)?;
        if list.checksum != checksum.trim() {
            return Err(DomainError::Conflict {
                reason: "the stored jurisdiction list checksum does not match its entries"
                    .to_string(),
            });
        }
        Ok(list)
    }

    /// The version label every decision cites.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// When the version was published.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// The entries in country-code order.
    #[must_use]
    pub fn entries(&self) -> &[JurisdictionListEntry] {
        &self.entries
    }

    /// How many countries the version names.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// The SHA-256 digest over the ordered entries.
    #[must_use]
    pub fn checksum(&self) -> &str {
        &self.checksum
    }

    /// A country's status. A country the list does not name is restricted: the
    /// gate fails closed (BR1.6).
    #[must_use]
    pub fn status_for(&self, code: &CountryCode) -> JurisdictionStatus {
        self.entries
            .binary_search_by(|entry| entry.code.cmp(code))
            .map(|index| self.entries[index].status)
            .unwrap_or(JurisdictionStatus::Restricted)
    }
}

fn validate_version(version: &str) -> Result<(), DomainError> {
    let valid = !version.is_empty()
        && version.len() <= MAX_LIST_VERSION_LEN
        && version.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        });
    if !valid {
        return Err(DomainError::Invalid {
            field: "version",
            reason: "must be 1-64 ASCII letters, digits, '.', '-', or '_'".to_string(),
        });
    }
    Ok(())
}

fn validate_entries(entries: &[JurisdictionListEntry]) -> Result<(), DomainError> {
    if entries.is_empty() {
        return Err(DomainError::Invalid {
            field: "entries",
            reason: "a list version must name at least one country".to_string(),
        });
    }
    for pair in entries.windows(2) {
        if pair[0].code == pair[1].code {
            return Err(DomainError::Invalid {
                field: "entries",
                reason: "a country code appears more than once".to_string(),
            });
        }
    }
    for entry in entries {
        let source = entry.source.trim();
        if source.is_empty()
            || source.len() > MAX_LIST_SOURCE_LEN
            || source.chars().any(char::is_control)
        {
            return Err(DomainError::Invalid {
                field: "source",
                reason: "must be 1-120 printable characters".to_string(),
            });
        }
    }
    Ok(())
}

fn checksum_of(entries: &[JurisdictionListEntry]) -> String {
    let mut hasher = Sha256::new();
    for entry in entries {
        hasher.update(entry.code.as_str().as_bytes());
        hasher.update(b"\t");
        hasher.update(entry.status.as_str().as_bytes());
        hasher.update(b"\t");
        hasher.update(entry.source.trim().as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

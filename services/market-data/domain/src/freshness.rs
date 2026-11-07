//! Freshness classification (BR2.1). A quote older than its tier bound is never
//! labelled live, and a delayed feed is labelled before a live one.

use crate::constants::HARD_BOUND_SECONDS;
use crate::error::DomainError;

/// The freshness a served quote carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Within the tier bound and from a live feed.
    Live,
    /// Within the tier bound and from a delayed feed.
    Delayed,
    /// Beyond the tier bound but within the hard bound.
    Stale,
    /// No observation may be served.
    Unpriced,
}

impl Freshness {
    /// The stored label and the C6 wire label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Delayed => "delayed",
            Self::Stale => "stale",
            Self::Unpriced => "unpriced",
        }
    }

    /// Parses a stored or wire label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "live" => Ok(Self::Live),
            "delayed" => Ok(Self::Delayed),
            "stale" => Ok(Self::Stale),
            "unpriced" => Ok(Self::Unpriced),
            _ => Err(DomainError::Invalid {
                field: "freshness",
                reason: "must be live, delayed, stale, or unpriced".to_string(),
            }),
        }
    }

    /// Whether the label carries a price, source, and timestamp.
    #[must_use]
    pub fn is_priced(self) -> bool {
        !matches!(self, Self::Unpriced)
    }
}

/// Classifies one observation's freshness.
///
/// The order is the rule's order: delayed before live, then stale, then
/// unpriced. An observation older than [`HARD_BOUND_SECONDS`] is never served
/// even though it exists.
#[must_use]
pub fn classify_freshness(
    age_seconds: i64,
    bound_seconds: i64,
    delayed_feed: bool,
    has_observation: bool,
) -> Freshness {
    if !has_observation {
        return Freshness::Unpriced;
    }
    if age_seconds <= bound_seconds {
        if delayed_feed {
            Freshness::Delayed
        } else {
            Freshness::Live
        }
    } else if age_seconds <= HARD_BOUND_SECONDS {
        Freshness::Stale
    } else {
        Freshness::Unpriced
    }
}

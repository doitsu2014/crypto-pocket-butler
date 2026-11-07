//! Append-only consent history (BR3.1, BR3.2). No purpose is granted by
//! default; the newest record per purpose is the current decision.

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{ConsentRecordId, UserId};

/// The consent purposes Identity records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConsentPurpose {
    /// Product analytics.
    Analytics,
    /// Marketing communication.
    Marketing,
    /// Launch disclosures.
    Disclosures,
}

impl ConsentPurpose {
    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "analytics" => Ok(Self::Analytics),
            "marketing" => Ok(Self::Marketing),
            "disclosures" => Ok(Self::Disclosures),
            _ => Err(DomainError::Invalid {
                field: "purpose",
                reason: "must be analytics, marketing, or disclosures".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analytics => "analytics",
            Self::Marketing => "marketing",
            Self::Disclosures => "disclosures",
        }
    }
}

/// Where a decision was recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConsentSource {
    /// Recorded during onboarding.
    #[default]
    Onboarding,
    /// Recorded in settings.
    Settings,
    /// Recorded through support.
    Support,
}

impl ConsentSource {
    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "onboarding" => Ok(Self::Onboarding),
            "settings" => Ok(Self::Settings),
            "support" => Ok(Self::Support),
            _ => Err(DomainError::Invalid {
                field: "source",
                reason: "must be onboarding, settings, or support".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Onboarding => "onboarding",
            Self::Settings => "settings",
            Self::Support => "support",
        }
    }
}

/// One recorded decision. Records are never edited or deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentRecord {
    id: ConsentRecordId,
    user_id: UserId,
    purpose: ConsentPurpose,
    granted: bool,
    recorded_at: DateTime<Utc>,
    source: ConsentSource,
}

impl ConsentRecord {
    /// Records a decision.
    #[must_use]
    pub fn record(
        id: ConsentRecordId,
        user_id: UserId,
        purpose: ConsentPurpose,
        granted: bool,
        source: ConsentSource,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            purpose,
            granted,
            recorded_at: now,
            source,
        }
    }

    /// The record identifier.
    #[must_use]
    pub fn id(&self) -> ConsentRecordId {
        self.id
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The purpose.
    #[must_use]
    pub fn purpose(&self) -> ConsentPurpose {
        self.purpose
    }

    /// Whether consent is granted.
    #[must_use]
    pub fn granted(&self) -> bool {
        self.granted
    }

    /// When the decision was recorded.
    #[must_use]
    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }

    /// Where the decision was recorded.
    #[must_use]
    pub fn source(&self) -> ConsentSource {
        self.source
    }
}

/// The newest analytics and marketing decisions (C3 `getConsent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentView {
    user_id: UserId,
    analytics: bool,
    marketing: bool,
}

impl ConsentView {
    /// Builds the view from the newest record per purpose, denying by default.
    #[must_use]
    pub fn from_latest(
        user_id: UserId,
        analytics: Option<&ConsentRecord>,
        marketing: Option<&ConsentRecord>,
    ) -> Self {
        Self {
            user_id,
            analytics: analytics.is_some_and(ConsentRecord::granted),
            marketing: marketing.is_some_and(ConsentRecord::granted),
        }
    }

    /// Builds the view from a whole history, selecting the newest per purpose.
    #[must_use]
    pub fn from_history(user_id: UserId, records: &[ConsentRecord]) -> Self {
        let mut analytics: Option<&ConsentRecord> = None;
        let mut marketing: Option<&ConsentRecord> = None;
        for record in records {
            if record.user_id != user_id {
                continue;
            }
            match record.purpose {
                ConsentPurpose::Analytics => {
                    if analytics.is_none_or(|current| record.recorded_at >= current.recorded_at) {
                        analytics = Some(record);
                    }
                }
                ConsentPurpose::Marketing => {
                    if marketing.is_none_or(|current| record.recorded_at >= current.recorded_at) {
                        marketing = Some(record);
                    }
                }
                ConsentPurpose::Disclosures => {}
            }
        }
        Self::from_latest(user_id, analytics, marketing)
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The newest analytics decision; absent means not granted.
    #[must_use]
    pub fn analytics(&self) -> bool {
        self.analytics
    }

    /// The newest marketing decision; absent means not granted.
    #[must_use]
    pub fn marketing(&self) -> bool {
        self.marketing
    }
}

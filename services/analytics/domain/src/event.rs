//! The closed event catalogue, per-event allow-lists, and the stored event
//! (BR2.1–BR2.3).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::consent::ConsentPurpose;
use crate::constants::{MAX_EVENT_NAME_LEN, MAX_PROPERTY_VALUE_LEN};
use crate::error::DomainError;
use crate::ids::AnalyticsEventId;
use crate::pii::detect_pii;

/// The consent class of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventClass {
    /// Requires a granted analytics consent.
    Analytics,
    /// Error reporting; always accepted.
    Essential,
}

impl EventClass {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analytics => "analytics",
            Self::Essential => "essential",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "analytics" => Ok(Self::Analytics),
            "essential" => Ok(Self::Essential),
            _ => Err(DomainError::Invalid {
                field: "eventClass",
                reason: "must be analytics or essential".to_string(),
            }),
        }
    }
}

/// One entry in the closed catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventDefinition {
    /// The event name.
    pub name: &'static str,
    /// Its consent class.
    pub class: EventClass,
    /// The only property keys that may be stored.
    pub allowed_properties: &'static [&'static str],
}

/// The closed catalogue from the functional specification.
pub const CATALOGUE: &[EventDefinition] = &[
    EventDefinition {
        name: "signup_started",
        class: EventClass::Analytics,
        allowed_properties: &["method"],
    },
    EventDefinition {
        name: "onboarding_step",
        class: EventClass::Analytics,
        allowed_properties: &["step"],
    },
    EventDefinition {
        name: "onboarding_completed",
        class: EventClass::Analytics,
        allowed_properties: &["path", "duration_bucket"],
    },
    EventDefinition {
        name: "valuation_read",
        class: EventClass::Analytics,
        allowed_properties: &["freshness", "asset_count_bucket"],
    },
    EventDefinition {
        name: "account_connected",
        class: EventClass::Analytics,
        allowed_properties: &["kind"],
    },
    EventDefinition {
        name: "drift_viewed",
        class: EventClass::Analytics,
        allowed_properties: &["portfolio_count_bucket"],
    },
    EventDefinition {
        name: "upgrade_started",
        class: EventClass::Analytics,
        allowed_properties: &["plan"],
    },
    EventDefinition {
        name: "support_opened",
        class: EventClass::Analytics,
        allowed_properties: &["entry"],
    },
    EventDefinition {
        name: "error_shown",
        class: EventClass::Essential,
        allowed_properties: &["code", "route"],
    },
];

/// Looks up a catalogue entry.
#[must_use]
pub fn definition(name: &str) -> Option<&'static EventDefinition> {
    CATALOGUE.iter().find(|entry| entry.name == name.trim())
}

/// The consent class of a catalogue event.
///
/// # Errors
/// Returns [`DomainError::UnknownEvent`] when the name is not in the catalogue.
pub fn event_class(name: &str) -> Result<EventClass, DomainError> {
    definition(name)
        .map(|entry| entry.class)
        .ok_or(DomainError::UnknownEvent)
}

/// An event whose properties have already been filtered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedEvent {
    event_name: String,
    class: EventClass,
    occurred_at: DateTime<Utc>,
    properties: BTreeMap<String, String>,
    dropped_properties: Vec<String>,
}

impl SanitizedEvent {
    /// The catalogue name.
    #[must_use]
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// The consent class.
    #[must_use]
    pub fn class(&self) -> EventClass {
        self.class
    }

    /// When the event happened.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// The allow-listed, PII-free properties.
    #[must_use]
    pub fn properties(&self) -> &BTreeMap<String, String> {
        &self.properties
    }

    /// Property names that were dropped; their values are never retained.
    #[must_use]
    pub fn dropped_properties(&self) -> &[String] {
        &self.dropped_properties
    }
}

/// BR2.1 and BR2.2: validates the event name, keeps only allow-listed
/// properties, and drops every PII-shaped value.
///
/// # Errors
/// Returns [`DomainError::UnknownEvent`] for an unknown name,
/// [`DomainError::PiiShaped`] when the offending value is `eventName` or
/// `occurredAt`, or a bounded-field error.
pub fn sanitize_event(
    event_name: &str,
    occurred_at: DateTime<Utc>,
    properties: BTreeMap<String, String>,
) -> Result<SanitizedEvent, DomainError> {
    let name = event_name.trim();
    if name.is_empty() {
        return Err(DomainError::Required { field: "eventName" });
    }
    if name.chars().count() > MAX_EVENT_NAME_LEN {
        return Err(DomainError::TooLarge {
            field: "eventName",
            limit: "the event-name limit",
        });
    }
    if detect_pii(name).is_some() {
        return Err(DomainError::PiiShaped { field: "eventName" });
    }
    let entry = definition(name).ok_or(DomainError::UnknownEvent)?;
    if detect_pii(&occurred_at.to_rfc3339()).is_some() {
        return Err(DomainError::PiiShaped {
            field: "occurredAt",
        });
    }

    let mut kept = BTreeMap::new();
    let mut dropped = Vec::new();
    for (key, value) in properties {
        if !entry.allowed_properties.contains(&key.as_str()) {
            dropped.push(key);
            continue;
        }
        let value = value.trim();
        if value.is_empty() || value.chars().count() > MAX_PROPERTY_VALUE_LEN {
            dropped.push(key);
            continue;
        }
        if detect_pii(value).is_some() {
            dropped.push(key);
            continue;
        }
        kept.insert(key, value.to_string());
    }

    Ok(SanitizedEvent {
        event_name: entry.name.to_string(),
        class: entry.class,
        occurred_at,
        properties: kept,
        dropped_properties: dropped,
    })
}

/// One stored, consented, PII-free event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyticsEvent {
    id: AnalyticsEventId,
    event_name: String,
    class: EventClass,
    occurred_at: DateTime<Utc>,
    properties: BTreeMap<String, String>,
    user_ref: Option<String>,
    consent_purpose: ConsentPurpose,
    ingested_at: DateTime<Utc>,
}

impl AnalyticsEvent {
    /// Builds a storable event from sanitized input.
    #[must_use]
    pub fn store(
        id: AnalyticsEventId,
        sanitized: SanitizedEvent,
        user_ref: Option<String>,
        ingested_at: DateTime<Utc>,
    ) -> Self {
        let consent_purpose = match sanitized.class {
            EventClass::Analytics => ConsentPurpose::Analytics,
            EventClass::Essential => ConsentPurpose::Essential,
        };
        Self {
            id,
            event_name: sanitized.event_name,
            class: sanitized.class,
            occurred_at: sanitized.occurred_at,
            properties: sanitized.properties,
            user_ref,
            consent_purpose,
            ingested_at,
        }
    }

    /// Rebuilds a stored row, re-checking the catalogue and purpose labels.
    ///
    /// # Errors
    /// Returns a domain error when a stored value no longer satisfies the
    /// catalogue.
    pub fn from_stored(
        id: AnalyticsEventId,
        event_name: &str,
        occurred_at: DateTime<Utc>,
        properties: BTreeMap<String, String>,
        user_ref: Option<String>,
        consent_purpose: &str,
        ingested_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let class = event_class(event_name)?;
        let expected = ConsentPurpose::Analytics;
        let purpose = ConsentPurpose::parse(consent_purpose)?;
        let matches_class = (class == EventClass::Analytics && purpose == expected)
            || (class == EventClass::Essential && purpose == ConsentPurpose::Essential);
        if !matches_class {
            return Err(DomainError::Invalid {
                field: "consentPurpose",
                reason: "does not match the catalogue class".to_string(),
            });
        }
        Ok(Self {
            id,
            event_name: event_name.to_string(),
            class,
            occurred_at,
            properties,
            user_ref,
            consent_purpose: purpose,
            ingested_at,
        })
    }

    /// The stored identifier.
    #[must_use]
    pub fn id(&self) -> AnalyticsEventId {
        self.id
    }

    /// The catalogue event name.
    #[must_use]
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// The consent class.
    #[must_use]
    pub fn class(&self) -> EventClass {
        self.class
    }

    /// When the event happened.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// The allow-listed properties.
    #[must_use]
    pub fn properties(&self) -> &BTreeMap<String, String> {
        &self.properties
    }

    /// The salted user hash, never a raw identifier.
    #[must_use]
    pub fn user_ref(&self) -> Option<&str> {
        self.user_ref.as_deref()
    }

    /// The purpose this event was stored under.
    #[must_use]
    pub fn consent_purpose(&self) -> ConsentPurpose {
        self.consent_purpose
    }

    /// When Analytics stored the event.
    #[must_use]
    pub fn ingested_at(&self) -> DateTime<Utc> {
        self.ingested_at
    }
}

//! Append-only consent history (BR3.1, BR3.2) and the U4 consent-event shape
//! (BR4.4–BR4.11). No purpose is granted by default; the newest event per kind
//! is the current decision.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{ConsentRecordId, UserId};
use crate::preferences::{BaseCurrency, Locale, TimeZone};

/// The longest policy-version label a consent event carries.
pub const MAX_POLICY_VERSION_LEN: usize = 120;
/// The longest stored IANA timezone name.
pub const MAX_TIMEZONE_LEN: usize = 64;

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

// ---------------------------------------------------------------------------
// U4 — consent events (BR4.4–BR4.8, BR4.10, BR4.11)
// ---------------------------------------------------------------------------

/// The consent kinds this release records (BR4.2, tech-stack decision).
///
/// `marketing` is its own kind: folding it into `analytics` would let a
/// marketing opt-out read as an analytics withdrawal, and a consent record is
/// evidence of what the person decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConsentKind {
    /// Launch terms accepted at the entry gates.
    Terms,
    /// The risk-disclosure acknowledgement.
    Risk,
    /// The self-declared age assurance (BR4.10).
    Age,
    /// Optional product analytics (BR4.11).
    Analytics,
    /// Optional marketing communication.
    Marketing,
}

impl ConsentKind {
    /// Every recorded kind, in the order the state answer lists them.
    pub const ALL: [Self; 5] = [
        Self::Terms,
        Self::Risk,
        Self::Age,
        Self::Analytics,
        Self::Marketing,
    ];

    /// The kinds a source connection requires (BR4.6).
    pub const REQUIRED: [Self; 3] = [Self::Terms, Self::Risk, Self::Age];

    /// Parses a stored or request value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything outside the five kinds.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "terms" => Ok(Self::Terms),
            "risk" => Ok(Self::Risk),
            "age" => Ok(Self::Age),
            "analytics" => Ok(Self::Analytics),
            "marketing" => Ok(Self::Marketing),
            _ => Err(DomainError::Invalid {
                field: "kind",
                reason: "must be terms, risk, age, analytics, or marketing".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terms => "terms",
            Self::Risk => "risk",
            Self::Age => "age",
            Self::Analytics => "analytics",
            Self::Marketing => "marketing",
        }
    }

    /// Whether a source connection requires this kind (BR4.6).
    #[must_use]
    pub fn is_required(self) -> bool {
        matches!(self, Self::Terms | Self::Risk | Self::Age)
    }
}

/// The event itself: a grant or a withdrawal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsentAction {
    /// The person accepted the kind.
    Granted,
    /// The person withdrew the kind.
    Withdrawn,
}

impl ConsentAction {
    /// Parses a stored or request value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything but the two actions.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "granted" => Ok(Self::Granted),
            "withdrawn" => Ok(Self::Withdrawn),
            _ => Err(DomainError::Invalid {
                field: "action",
                reason: "must be granted or withdrawn".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Withdrawn => "withdrawn",
        }
    }
}

/// The current state of one kind, derived from its newest event (BR4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsentState {
    /// The person has recorded nothing for the kind.
    None,
    /// The newest event is a grant.
    Granted,
    /// The newest event is a withdrawal.
    Withdrawn,
}

impl ConsentState {
    /// The value the state answer and the history expose.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Granted => "granted",
            Self::Withdrawn => "withdrawn",
        }
    }
}

impl From<ConsentAction> for ConsentState {
    fn from(action: ConsentAction) -> Self {
        match action {
            ConsentAction::Granted => Self::Granted,
            ConsentAction::Withdrawn => Self::Withdrawn,
        }
    }
}

/// The version of the text the person acted on (BR4.8).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConsentPolicy(String);

impl ConsentPolicy {
    /// Validates and trims a policy version.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank value and
    /// [`DomainError::TooLarge`] above 120 characters.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(DomainError::Required {
                field: "policyVersion",
            });
        }
        if value.chars().count() > MAX_POLICY_VERSION_LEN {
            return Err(DomainError::TooLarge {
                field: "policyVersion",
                limit: "120 characters",
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

/// A consent event that has not been stored yet. The database assigns the
/// identifier, the sequence, and the timestamp (BR4.5, BR4.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentEventDraft {
    user_id: UserId,
    kind: ConsentKind,
    action: ConsentAction,
    policy_version: ConsentPolicy,
}

impl ConsentEventDraft {
    /// Records one decision for one kind.
    #[must_use]
    pub fn new(
        user_id: UserId,
        kind: ConsentKind,
        action: ConsentAction,
        policy_version: ConsentPolicy,
    ) -> Self {
        Self {
            user_id,
            kind,
            action,
            policy_version,
        }
    }

    /// A grant draft.
    #[must_use]
    pub fn grant(user_id: UserId, kind: ConsentKind, policy_version: ConsentPolicy) -> Self {
        Self::new(user_id, kind, ConsentAction::Granted, policy_version)
    }

    /// A withdrawal draft. The policy version is the prior grant's, resolved by
    /// the caller from the newest grant (BR4.8).
    #[must_use]
    pub fn withdraw(user_id: UserId, kind: ConsentKind, policy_version: ConsentPolicy) -> Self {
        Self::new(user_id, kind, ConsentAction::Withdrawn, policy_version)
    }

    /// The consenting user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The kind.
    #[must_use]
    pub fn kind(&self) -> ConsentKind {
        self.kind
    }

    /// The action.
    #[must_use]
    pub fn action(&self) -> ConsentAction {
        self.action
    }

    /// The policy version.
    #[must_use]
    pub fn policy_version(&self) -> &ConsentPolicy {
        &self.policy_version
    }
}

/// One stored consent event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentEvent {
    id: ConsentRecordId,
    seq: i64,
    user_id: UserId,
    kind: ConsentKind,
    action: ConsentAction,
    occurred_at: DateTime<Utc>,
    policy_version: String,
}

impl ConsentEvent {
    /// Rebuilds a stored event.
    #[must_use]
    pub fn from_stored(
        id: ConsentRecordId,
        seq: i64,
        user_id: UserId,
        kind: ConsentKind,
        action: ConsentAction,
        occurred_at: DateTime<Utc>,
        policy_version: impl Into<String>,
    ) -> Self {
        Self {
            id,
            seq,
            user_id,
            kind,
            action,
            occurred_at,
            policy_version: policy_version.into(),
        }
    }

    /// The event identifier.
    #[must_use]
    pub fn id(&self) -> ConsentRecordId {
        self.id
    }

    /// The database-generated monotonic sequence, the ordering tie-breaker.
    #[must_use]
    pub fn seq(&self) -> i64 {
        self.seq
    }

    /// The consenting user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The kind.
    #[must_use]
    pub fn kind(&self) -> ConsentKind {
        self.kind
    }

    /// The action.
    #[must_use]
    pub fn action(&self) -> ConsentAction {
        self.action
    }

    /// When the database recorded the event.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// The policy version the person acted on.
    #[must_use]
    pub fn policy_version(&self) -> &str {
        &self.policy_version
    }
}

/// The consent state per kind and the kinds a source connection still needs
/// (BR4.5, BR4.6, BR4.11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentStateView {
    user_id: UserId,
    kinds: BTreeMap<ConsentKind, ConsentState>,
}

impl ConsentStateView {
    /// Builds the view from the newest event per kind. A kind with no event is
    /// [`ConsentState::None`].
    #[must_use]
    pub fn from_newest(user_id: UserId, newest: impl IntoIterator<Item = ConsentEvent>) -> Self {
        let mut kinds: BTreeMap<ConsentKind, ConsentState> = ConsentKind::ALL
            .into_iter()
            .map(|kind| (kind, ConsentState::None))
            .collect();
        for event in newest {
            kinds.insert(event.kind(), ConsentState::from(event.action()));
        }
        Self { user_id, kinds }
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The state of one kind.
    #[must_use]
    pub fn state(&self, kind: ConsentKind) -> ConsentState {
        self.kinds.get(&kind).copied().unwrap_or(ConsentState::None)
    }

    /// The state of every kind.
    #[must_use]
    pub fn kinds(&self) -> &BTreeMap<ConsentKind, ConsentState> {
        &self.kinds
    }

    /// Whether one kind is currently granted.
    #[must_use]
    pub fn is_granted(&self, kind: ConsentKind) -> bool {
        self.state(kind) == ConsentState::Granted
    }

    /// Every required kind that is not currently granted, in `REQUIRED` order.
    /// `analytics` and `marketing` never appear (BR4.11).
    #[must_use]
    pub fn missing(&self) -> Vec<ConsentKind> {
        ConsentKind::REQUIRED
            .into_iter()
            .filter(|kind| !self.is_granted(*kind))
            .collect()
    }

    /// Whether any required kind is not granted.
    #[must_use]
    pub fn has_missing(&self) -> bool {
        ConsentKind::REQUIRED
            .into_iter()
            .any(|kind| !self.is_granted(kind))
    }
}

// ---------------------------------------------------------------------------
// U4 — preferences (BR4.1–BR4.3, BR4.9)
// ---------------------------------------------------------------------------

/// The closed base-currency set (BR4.2). A provider-derived code outside the
/// set falls back to the default rather than being echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BaseCurrencyCode {
    /// United States dollar.
    #[default]
    Usd,
    /// Euro.
    Eur,
    /// Pound sterling.
    Gbp,
    /// Vietnamese đồng.
    Vnd,
}

impl BaseCurrencyCode {
    /// Every allowed code.
    pub const ALL: [Self; 4] = [Self::Usd, Self::Eur, Self::Gbp, Self::Vnd];

    /// Parses an exact upper-case code from the closed set.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything outside the four codes.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "USD" => Ok(Self::Usd),
            "EUR" => Ok(Self::Eur),
            "GBP" => Ok(Self::Gbp),
            "VND" => Ok(Self::Vnd),
            _ => Err(DomainError::Invalid {
                field: "baseCurrency",
                reason: "must be USD, EUR, GBP, or VND".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Eur => "EUR",
            Self::Gbp => "GBP",
            Self::Vnd => "VND",
        }
    }
}

/// Validates an IANA timezone name against the `chrono-tz` registry. The name is
/// stored and returned unchanged (NFR16.1).
///
/// # Errors
/// Returns [`DomainError::Invalid`] when the name is not a known IANA zone.
pub fn parse_iana_timezone(raw: &str) -> Result<TimeZone, DomainError> {
    let value = raw.trim();
    let invalid = || DomainError::Invalid {
        field: "timezone",
        reason: "must be a known IANA timezone name".to_string(),
    };
    if value.is_empty() || value.chars().count() > MAX_TIMEZONE_LEN {
        return Err(invalid());
    }
    value.parse::<chrono_tz::Tz>().map_err(|_| invalid())?;
    TimeZone::parse(value).map_err(|_| invalid())
}

/// Where an effective preference value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreferenceSource {
    /// The user's explicit choice, stored in `user_preferences`.
    User,
    /// A default: u1's provider-derived profile value or the literal fallback.
    Default,
}

impl PreferenceSource {
    /// The value the read exposes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Default => "default",
        }
    }
}

/// One effective preference value and its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceValue<T> {
    value: T,
    source: PreferenceSource,
}

impl<T> PreferenceValue<T> {
    /// The user's explicit choice.
    #[must_use]
    pub fn from_user(value: T) -> Self {
        Self {
            value,
            source: PreferenceSource::User,
        }
    }

    /// A default, never written over a choice (BR4.1).
    #[must_use]
    pub fn from_default(value: T) -> Self {
        Self {
            value,
            source: PreferenceSource::Default,
        }
    }

    /// The effective value.
    #[must_use]
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Where the value came from.
    #[must_use]
    pub fn source(&self) -> PreferenceSource {
        self.source
    }
}

/// The stored explicit choices. A `None` field is a preference the user has
/// never set (BR4.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredPreferences {
    /// The chosen UI language.
    pub locale: Option<Locale>,
    /// The chosen base currency.
    pub base_currency: Option<BaseCurrencyCode>,
    /// The chosen IANA timezone.
    pub timezone: Option<TimeZone>,
    /// Whether email notifications are enabled.
    pub notification_email: Option<bool>,
    /// Whether push notifications are enabled.
    pub notification_push: Option<bool>,
}

impl StoredPreferences {
    /// A row holding no explicit choice.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

/// A partial write: only the supplied fields change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreferenceUpdate {
    /// The new locale, when supplied.
    pub locale: Option<Locale>,
    /// The new base currency, when supplied.
    pub base_currency: Option<BaseCurrencyCode>,
    /// The new IANA timezone, when supplied.
    pub timezone: Option<TimeZone>,
    /// The new email-notification choice, when supplied.
    pub notification_email: Option<bool>,
    /// The new push-notification choice, when supplied.
    pub notification_push: Option<bool>,
}

impl PreferenceUpdate {
    /// Whether the update supplies no field at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.locale.is_none()
            && self.base_currency.is_none()
            && self.timezone.is_none()
            && self.notification_email.is_none()
            && self.notification_push.is_none()
    }
}

/// u1's provider-derived profile values, used only when the user has no
/// explicit choice. U4 never writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfile {
    /// The provider-derived locale.
    pub locale: Locale,
    /// The provider-derived base currency.
    pub base_currency: BaseCurrency,
    /// The provider-derived timezone.
    pub timezone: TimeZone,
}

impl ProviderProfile {
    /// Wraps the three stored profile values.
    #[must_use]
    pub fn new(locale: Locale, base_currency: BaseCurrency, timezone: TimeZone) -> Self {
        Self {
            locale,
            base_currency,
            timezone,
        }
    }
}

/// The effective preferences returned to a caller, each value with its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePreferences {
    user_id: UserId,
    locale: PreferenceValue<Locale>,
    base_currency: PreferenceValue<BaseCurrencyCode>,
    timezone: PreferenceValue<TimeZone>,
    notification_email: PreferenceValue<bool>,
    notification_push: PreferenceValue<bool>,
}

impl EffectivePreferences {
    /// Resolves `COALESCE(preference, provider, default)` per field. A provider
    /// value outside the preference's closed set falls back to the literal
    /// default rather than being echoed.
    #[must_use]
    pub fn resolve(
        user_id: UserId,
        stored: Option<&StoredPreferences>,
        provider: &ProviderProfile,
    ) -> Self {
        let locale = match stored.and_then(|stored| stored.locale) {
            Some(value) => PreferenceValue::from_user(value),
            None => PreferenceValue::from_default(provider.locale),
        };
        let base_currency = match stored.and_then(|stored| stored.base_currency) {
            Some(value) => PreferenceValue::from_user(value),
            None => PreferenceValue::from_default(
                BaseCurrencyCode::parse(provider.base_currency.as_str()).unwrap_or_default(),
            ),
        };
        let timezone = match stored.and_then(|stored| stored.timezone.clone()) {
            Some(value) => PreferenceValue::from_user(value),
            None => PreferenceValue::from_default(
                parse_iana_timezone(provider.timezone.as_str())
                    .unwrap_or_else(|_| TimeZone::default()),
            ),
        };
        let notification_email = match stored.and_then(|stored| stored.notification_email) {
            Some(value) => PreferenceValue::from_user(value),
            None => PreferenceValue::from_default(false),
        };
        let notification_push = match stored.and_then(|stored| stored.notification_push) {
            Some(value) => PreferenceValue::from_user(value),
            None => PreferenceValue::from_default(false),
        };
        Self {
            user_id,
            locale,
            base_currency,
            timezone,
            notification_email,
            notification_push,
        }
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The effective locale.
    #[must_use]
    pub fn locale(&self) -> &PreferenceValue<Locale> {
        &self.locale
    }

    /// The effective base currency.
    #[must_use]
    pub fn base_currency(&self) -> &PreferenceValue<BaseCurrencyCode> {
        &self.base_currency
    }

    /// The effective timezone.
    #[must_use]
    pub fn timezone(&self) -> &PreferenceValue<TimeZone> {
        &self.timezone
    }

    /// The effective email-notification choice.
    #[must_use]
    pub fn notification_email(&self) -> &PreferenceValue<bool> {
        &self.notification_email
    }

    /// The effective push-notification choice.
    #[must_use]
    pub fn notification_push(&self) -> &PreferenceValue<bool> {
        &self.notification_push
    }
}

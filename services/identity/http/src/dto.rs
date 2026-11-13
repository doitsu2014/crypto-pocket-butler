//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from domain and application types here; no DTO type reaches inward.
//!
//! The support-access DTOs carry the closed metadata allowlist (FR16.5, BR3.9)
//! and the grant shape (FR16.3–FR16.6).

use chrono::{DateTime, Utc};
use identity_application::account_lifecycle::AccountStatusView;
use identity_application::common::{FieldError, PrincipalView};
use identity_application::jurisdiction::{
    CheckResult, PublishEntry, PublishRequest, PublishedList,
};
use identity_application::ports::GateGrant;
use identity_application::profile::ProfileView;
use identity_application::support::SupportMetadata;
use identity_application::user_registry::{AdminUserView, UserRecordView};
use identity_domain::{
    parse_iana_timezone, AssuranceLevel, BaseCurrency, BaseCurrencyCode, ConsentPurpose,
    ConsentRecord, ConsentSource, ConsentView, DeletionRequest, DeviceSession, DomainError,
    EffectivePreferences, ExportRequest, Locale, PreferenceUpdate, Preferences, Role,
    SupportAccessGrant, TimeZone, User, UserStatus, VerifiedClaims,
};
use serde::{Deserialize, Serialize};

/// `POST /v1/signup-gate` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupGateRequest {
    /// The declared ISO 3166-1 alpha-2 code.
    pub jurisdiction: String,
    /// The declared age.
    pub declared_age: u32,
    /// Whether the launch disclosures were acknowledged.
    #[serde(default)]
    pub disclosures_acknowledged: bool,
}

/// `POST /v1/signup-gate` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupGateResponse {
    /// The signed gate token, absent while disclosures are outstanding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_token: Option<String>,
    /// Whether the disclosures must be acknowledged first.
    pub disclosures_required: bool,
}

impl SignupGateResponse {
    /// Builds the response from the application answer.
    #[must_use]
    pub fn from_tokens(gate_token: Option<String>, disclosures_required: bool) -> Self {
        Self {
            gate_token,
            disclosures_required,
        }
    }
}

/// `GET /v1/principals/{authUserId}` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrincipalResponse {
    /// The internal user identifier.
    pub user_id: String,
    /// The capability role.
    pub role: String,
    /// The lifecycle state.
    pub status: String,
    /// What the caller must do next.
    pub next_step: String,
}

impl From<PrincipalView> for PrincipalResponse {
    fn from(view: PrincipalView) -> Self {
        Self {
            user_id: view.user_id().to_string(),
            role: view.role().as_str().to_string(),
            status: view.status().as_str().to_string(),
            next_step: view.next_step().as_str().to_string(),
        }
    }
}

/// The C3 metadata boundary for one user record. It never carries the provider
/// subject, a credential, a token, or any portfolio content.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    /// The internal user identifier.
    pub id: String,
    /// The display name, when one is stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The stored address, when one is stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Whether the stored address is confirmed.
    pub email_verified: bool,
    /// The lifecycle state.
    pub status: String,
    /// When the record was created.
    pub created_at: DateTime<Utc>,
    /// When the record last changed.
    pub updated_at: DateTime<Utc>,
}

impl From<UserRecordView> for UserResponse {
    fn from(view: UserRecordView) -> Self {
        Self {
            id: view.user_id().to_string(),
            display_name: view.display_name().map(ToString::to_string),
            email: view.email().map(ToString::to_string),
            email_verified: view.email_verified(),
            status: view.status().as_str().to_string(),
            created_at: view.created_at(),
            updated_at: view.updated_at(),
        }
    }
}

/// `POST /v1/admin/users/lookup` body (security design D-S1).
///
/// The address travels in the body, never in the path or a query string, so it
/// never reaches a proxy access log.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLookupRequest {
    /// The exact address to look up.
    #[serde(default)]
    pub email: String,
}

/// The U10 admin lookup answer: the U1 record plus the provider's role.
///
/// The provider subject is never serialized here.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserResponse {
    /// The internal user identifier.
    pub id: String,
    /// The display name, when one is stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The stored address, when one is stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Whether the stored address is confirmed.
    pub email_verified: bool,
    /// The lifecycle state.
    pub status: String,
    /// When the record was created.
    pub created_at: DateTime<Utc>,
    /// When the record last changed.
    pub updated_at: DateTime<Utc>,
    /// The role the provider currently reports (`owner`, `admin`, `support`).
    pub role: String,
}

impl From<AdminUserView> for AdminUserResponse {
    fn from(view: AdminUserView) -> Self {
        let record = view.record();
        Self {
            id: record.user_id().to_string(),
            display_name: record.display_name().map(ToString::to_string),
            email: record.email().map(ToString::to_string),
            email_verified: record.email_verified(),
            status: record.status().as_str().to_string(),
            created_at: record.created_at(),
            updated_at: record.updated_at(),
            role: view.role().provider_value().to_string(),
        }
    }
}

/// `PUT /v1/internal/users/by-subject/{subjectHash}` body (C3).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalProvisionRequest {
    /// The plaintext provider subject the caller validated. It is never logged
    /// and never returned; the registry stores only its hash and ciphertext.
    pub subject: String,
    /// `user`, `admin`, or `support`; defaults to `user`.
    #[serde(default)]
    pub role: Option<String>,
    /// The address the provider carries, when it carries one.
    #[serde(default)]
    pub email: Option<String>,
    /// Whether the provider confirms the address.
    #[serde(default)]
    pub email_confirmed: bool,
    /// The display name the provider supplies.
    #[serde(default)]
    pub display_name: Option<String>,
    /// `aal1` or `aal2`.
    #[serde(default)]
    pub assurance: Option<String>,
    /// Whether the caller holds exchange credentials.
    #[serde(default)]
    pub has_exchange_credentials: bool,
    /// The self-declared ISO 3166-1 alpha-2 country, when the caller carries it.
    #[serde(default)]
    pub country_code: Option<String>,
    /// The observed client address, when the caller carries it.
    #[serde(default)]
    pub client_ip: Option<String>,
}

impl InternalProvisionRequest {
    /// Builds the validated-claims view the registry acts on. The trusted
    /// caller's headers take precedence over the body for the declaration.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown role.
    pub fn into_claims(
        self,
        country_header: Option<&str>,
        ip_header: Option<&str>,
    ) -> Result<VerifiedClaims, DomainError> {
        if self.subject.trim().is_empty() {
            return Err(DomainError::Required { field: "subject" });
        }
        if self.subject.chars().count() > 200 {
            return Err(DomainError::TooLarge {
                field: "subject",
                limit: "200 characters",
            });
        }
        let role = match self.role.as_deref() {
            Some(raw) => Role::parse(raw)?,
            None => Role::User,
        };
        Ok(VerifiedClaims {
            subject: self.subject,
            role,
            email_confirmed: self.email_confirmed,
            email: self.email.map(|email| email.trim().to_string()),
            display_name: self.display_name,
            assurance: AssuranceLevel::parse(self.assurance.as_deref()),
            has_exchange_credentials: self.has_exchange_credentials,
            session_id: None,
            user_agent: None,
            location: None,
            country_code: country_header
                .map(ToString::to_string)
                .or(self.country_code),
            client_ip: ip_header.map(ToString::to_string).or(self.client_ip),
        })
    }
}

/// `POST /v1/consent` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentAppendRequest {
    /// `analytics`, `marketing`, or `disclosures`.
    pub purpose: String,
    /// The decision.
    pub granted: bool,
    /// `onboarding`, `settings`, or `support`; defaults to onboarding.
    #[serde(default)]
    pub source: Option<String>,
}

impl ConsentAppendRequest {
    /// Parses the purpose at the boundary.
    ///
    /// # Errors
    /// Returns [`DomainError`] for an unknown purpose.
    pub fn purpose(&self) -> Result<ConsentPurpose, DomainError> {
        ConsentPurpose::parse(&self.purpose)
    }

    /// Parses the source at the boundary.
    ///
    /// # Errors
    /// Returns [`DomainError`] for an unknown source.
    pub fn source(&self) -> Result<ConsentSource, DomainError> {
        ConsentSource::parse(self.source.as_deref().unwrap_or("onboarding"))
    }
}

/// `POST /v1/consent` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentAppendResponse {
    /// The record identifier.
    pub id: String,
    /// The purpose.
    pub purpose: String,
    /// The decision.
    pub granted: bool,
    /// When the decision was recorded.
    pub recorded_at: DateTime<Utc>,
    /// Where the decision was recorded.
    pub source: String,
}

impl From<&ConsentRecord> for ConsentAppendResponse {
    fn from(record: &ConsentRecord) -> Self {
        Self {
            id: record.id().to_string(),
            purpose: record.purpose().as_str().to_string(),
            granted: record.granted(),
            recorded_at: record.recorded_at(),
            source: record.source().as_str().to_string(),
        }
    }
}

/// `GET /v1/consent/{userId}` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentResponse {
    /// The owner.
    pub user_id: String,
    /// The newest analytics decision.
    pub analytics: bool,
    /// The newest marketing decision.
    pub marketing: bool,
}

impl From<ConsentView> for ConsentResponse {
    fn from(view: ConsentView) -> Self {
        Self {
            user_id: view.user_id().to_string(),
            analytics: view.analytics(),
            marketing: view.marketing(),
        }
    }
}

/// `GET /v1/users/{userId}/contact` response (C3).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactResponse {
    /// The user the answer concerns.
    pub user_id: String,
    /// The verified address, absent when there is none or it is unconfirmed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Whether a confirmed address exists.
    pub verified: bool,
}

impl From<identity_application::ContactView> for ContactResponse {
    fn from(view: identity_application::ContactView) -> Self {
        Self {
            user_id: view.user_id.to_string(),
            email: view.email,
            verified: view.verified,
        }
    }
}

/// `PATCH /v1/users/{userId}/preferences` body. Omitted fields keep the stored
/// value.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesRequest {
    /// `en` or `vi`.
    #[serde(default)]
    pub locale: Option<String>,
    /// A three-letter ISO 4217 code.
    #[serde(default)]
    pub base_currency: Option<String>,
    /// An IANA timezone name.
    #[serde(default)]
    pub timezone: Option<String>,
}

impl PreferencesRequest {
    /// Validates the supplied fields against the U4 closed sets and builds a
    /// partial update against `user_preferences`. Omitted fields are untouched,
    /// so a write through the legacy path can never be read back as a default.
    ///
    /// # Errors
    /// Returns every field problem at once.
    pub fn into_update(self) -> Result<PreferenceUpdate, Vec<FieldError>> {
        let mut errors = Vec::new();
        let locale = match self.locale {
            Some(raw) => match Locale::parse(&raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(field(error));
                    None
                }
            },
            None => None,
        };
        let base_currency = match self.base_currency {
            Some(raw) => match BaseCurrencyCode::parse(&raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(field(error));
                    None
                }
            },
            None => None,
        };
        let timezone = match self.timezone {
            Some(raw) => match parse_iana_timezone(&raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(field(error));
                    None
                }
            },
            None => None,
        };
        if errors.is_empty() {
            Ok(PreferenceUpdate {
                locale,
                base_currency,
                timezone,
                notification_email: None,
                notification_push: None,
            })
        } else {
            Err(errors)
        }
    }

    /// Validates the supplied fields and fills the omitted ones from `current`.
    ///
    /// # Errors
    /// Returns every field problem at once.
    pub fn into_preferences(self, current: &User) -> Result<Preferences, Vec<FieldError>> {
        let mut errors = Vec::new();
        let locale = match self.locale {
            Some(raw) => match Locale::parse(&raw) {
                Ok(value) => value,
                Err(error) => {
                    errors.push(field(error));
                    current.locale()
                }
            },
            None => current.locale(),
        };
        let base_currency = match self.base_currency {
            Some(raw) => match BaseCurrency::parse(&raw) {
                Ok(value) => value,
                Err(error) => {
                    errors.push(field(error));
                    current.base_currency()
                }
            },
            None => current.base_currency(),
        };
        let timezone = match self.timezone {
            Some(raw) => match TimeZone::parse(&raw) {
                Ok(value) => value,
                Err(error) => {
                    errors.push(field(error));
                    current.timezone()
                }
            },
            None => current.timezone(),
        };
        if errors.is_empty() {
            Ok(Preferences {
                locale,
                base_currency,
                timezone,
            })
        } else {
            Err(errors)
        }
    }
}

fn field(error: DomainError) -> FieldError {
    FieldError::new(error.field(), error.to_string())
}

/// `PATCH .../preferences` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesResponse {
    /// The owner.
    pub user_id: String,
    /// The stored locale.
    pub locale: String,
    /// The stored base currency.
    pub base_currency: String,
    /// The stored timezone.
    pub timezone: String,
}

impl From<&User> for PreferencesResponse {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.id().to_string(),
            locale: user.locale().as_str().to_string(),
            base_currency: user.base_currency().as_str().to_string(),
            timezone: user.timezone().as_str().to_string(),
        }
    }
}

impl PreferencesResponse {
    /// Builds the legacy flat response from the effective `user_preferences`
    /// values, so the retired-owner route returns the same shape without reading
    /// u1's provider-derived columns.
    #[must_use]
    pub fn from_effective(effective: &EffectivePreferences) -> Self {
        Self {
            user_id: effective.user_id().to_string(),
            locale: effective.locale().value().as_str().to_string(),
            base_currency: effective.base_currency().value().as_str().to_string(),
            timezone: effective.timezone().value().as_str().to_string(),
        }
    }
}

/// One session row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    /// The row identifier.
    pub id: String,
    /// A short client label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Coarse city/country, never the full IP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// The last observed activity.
    pub last_activity_at: DateTime<Utc>,
    /// When the session was revoked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    /// Whether this is the session making the request.
    pub current: bool,
}

impl SessionView {
    /// Maps one stored row.
    #[must_use]
    pub fn from_session(session: &DeviceSession, current_session_id: Option<&str>) -> Self {
        Self {
            id: session.id().to_string(),
            device: session.device().map(ToString::to_string),
            location: session.location().map(ToString::to_string),
            last_activity_at: session.last_activity_at(),
            revoked_at: session.revoked_at(),
            current: current_session_id == Some(session.auth_session_id()),
        }
    }
}

/// `GET .../sessions` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionsResponse {
    /// The session rows.
    pub sessions: Vec<SessionView>,
}

/// `POST /v1/exports` and `GET /v1/exports/{exportId}` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    /// The request identifier.
    pub export_id: String,
    /// The lifecycle state.
    pub status: String,
    /// Percentage of contributors that answered.
    pub progress: u8,
    /// When the request arrived.
    pub requested_at: DateTime<Utc>,
    /// When an unfinished request expires.
    pub assembly_deadline: DateTime<Utc>,
    /// When the signed download stops working.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_expires_at: Option<DateTime<Utc>>,
    /// Why the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// The owner-only signed URL, while the window is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_download: Option<String>,
}

impl ExportResponse {
    /// Maps the stored request plus an optional signed download.
    #[must_use]
    pub fn from_export(export: &ExportRequest, signed_download: Option<String>) -> Self {
        Self {
            export_id: export.id().to_string(),
            status: export.status().as_str().to_string(),
            progress: export.progress(),
            requested_at: export.requested_at(),
            assembly_deadline: export.assembly_deadline(),
            download_expires_at: export.download_expires_at(),
            failure_reason: export.failure_reason().map(ToString::to_string),
            signed_download,
        }
    }
}

/// `POST /v1/privacy/deletions` and `GET /v1/privacy/deletions/{deletionId}`
/// response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionResponse {
    /// The request identifier.
    pub deletion_id: String,
    /// The lifecycle state.
    pub status: String,
    /// When the deletion was requested.
    pub requested_at: DateTime<Utc>,
    /// When the deletion completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Why the last attempt failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// The labelled retention categories.
    pub retained_categories: Vec<RetainedCategoryView>,
    /// The end of the recovery window (`recoveryEndsAt`, C7/W2).
    pub recovery_ends_at: DateTime<Utc>,
    /// When the owner restored, if they did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<DateTime<Utc>>,
    /// When the erasure completed, if it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erased_at: Option<DateTime<Utc>>,
    /// Failed erasure attempts so far.
    pub attempt_count: u32,
}

/// One retained category.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetainedCategoryView {
    /// The category label.
    pub category: String,
    /// The legal basis label.
    pub basis: String,
}

impl From<&DeletionRequest> for DeletionResponse {
    fn from(deletion: &DeletionRequest) -> Self {
        Self {
            deletion_id: deletion.id().to_string(),
            status: deletion.status().as_str().to_string(),
            requested_at: deletion.requested_at(),
            completed_at: deletion.completed_at(),
            failure_reason: deletion.failure_reason().map(ToString::to_string),
            retained_categories: deletion
                .retained_categories()
                .iter()
                .map(|category| RetainedCategoryView {
                    category: category.category().to_string(),
                    basis: category.basis().to_string(),
                })
                .collect(),
            recovery_ends_at: deletion.recoverable_until(),
            cancelled_at: deletion.cancelled_at(),
            erased_at: deletion.erased_at(),
            attempt_count: deletion.attempt_count(),
        }
    }
}

/// `POST /internal/users/{userId}/disable` and `/restore` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStatusRequest {
    /// The recorded reason (1–500 characters, BR5.2).
    #[serde(default)]
    pub reason: String,
}

/// The answer to a disable or restore (C7).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatusResponse {
    /// The account the change concerns.
    pub user_id: String,
    /// The status after the change.
    pub status: String,
    /// The recorded transition.
    pub transition_id: String,
    /// When the change was recorded.
    pub occurred_at: DateTime<Utc>,
}

impl From<&AccountStatusView> for AccountStatusResponse {
    fn from(view: &AccountStatusView) -> Self {
        Self {
            user_id: view.user_id().to_string(),
            status: view.status().as_str().to_string(),
            transition_id: view.transition().id().to_string(),
            occurred_at: view.transition().occurred_at(),
        }
    }
}

/// `POST /v1/support-grants` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportGrantRequest {
    /// The target user.
    pub user_id: String,
    /// The recorded reason.
    pub reason: String,
}

/// One support-grant response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportGrantResponse {
    /// The grant identifier.
    pub grant_id: String,
    /// The target user.
    pub user_id: String,
    /// The lifecycle state.
    pub status: String,
    /// The recorded reason.
    pub reason: String,
    /// When the grant was requested.
    pub requested_at: DateTime<Utc>,
    /// The distinct approver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    /// When the grant was approved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<DateTime<Utc>>,
    /// When the window ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<&SupportAccessGrant> for SupportGrantResponse {
    fn from(grant: &SupportAccessGrant) -> Self {
        Self {
            grant_id: grant.id().to_string(),
            user_id: grant.user_id().to_string(),
            status: grant.status().as_str().to_string(),
            reason: grant.reason().to_string(),
            requested_at: grant.requested_at(),
            approved_by: grant.approved_by().map(|id| id.to_string()),
            approved_at: grant.approved_at(),
            expires_at: grant.expires_at(),
        }
    }
}

/// One entry of the pending-grant list (`GET /v1/support-grants`, W5) and of
/// the support agent's own bare-array read (`GET /v1/support-grants/mine`, S2).
///
/// The C5 grant shape: the grant id, the target, the requester and the
/// timestamps the console's approvals panel reads. `requestedBy` is the agent
/// who asked, so an own-grant row is attributable without the caller echoing
/// their own id.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingGrantResponse {
    /// The grant identifier.
    pub id: String,
    /// The target user.
    pub user_id: String,
    /// The support agent who requested the grant.
    pub requested_by: String,
    /// The recorded reason.
    pub reason: String,
    /// The lifecycle state.
    pub status: String,
    /// When the grant was requested.
    pub requested_at: DateTime<Utc>,
    /// The distinct approver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    /// When the grant was approved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<DateTime<Utc>>,
    /// When the window ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<&SupportAccessGrant> for PendingGrantResponse {
    fn from(grant: &SupportAccessGrant) -> Self {
        Self {
            id: grant.id().to_string(),
            user_id: grant.user_id().to_string(),
            requested_by: grant.support_agent_id().to_string(),
            reason: grant.reason().to_string(),
            status: grant.status().as_str().to_string(),
            requested_at: grant.requested_at(),
            approved_by: grant.approved_by().map(|id| id.to_string()),
            approved_at: grant.approved_at(),
            expires_at: grant.expires_at(),
        }
    }
}

/// The closed support-metadata projection the grant holder reads (FR16.5,
/// BR3.9). Every field here is on the allowlist; an omitted field is omitted by
/// construction, not filtered at render time.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportMetadataResponse {
    /// The user record the grant names.
    pub user_id: String,
    /// The display name, serialized as `null` when genuinely absent.
    pub display_name: Option<String>,
    /// Verification status only — never the email address.
    pub email_verified: bool,
    /// The allowlisted account summaries.
    pub accounts: Vec<SupportAccountDto>,
    /// The allowlisted portfolio summaries.
    pub portfolios: Vec<SupportPortfolioDto>,
}

/// One allowlisted account summary (FR16.5, BR3.9).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportAccountDto {
    /// The account identifier.
    pub id: String,
    /// The account label.
    pub label: String,
    /// `exchange`, `wallet`, `walletconnect`, or `csv`.
    pub source_type: String,
    /// `active`, `syncing`, `error`, or `disconnected`.
    pub sync_status: String,
    /// When the account last synced; `null` when it never has.
    pub last_synced_at: Option<DateTime<Utc>>,
}

/// One allowlisted portfolio summary (FR16.5, BR3.9).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPortfolioDto {
    /// The portfolio identifier.
    pub id: String,
    /// The portfolio label.
    pub label: String,
}

impl From<&SupportMetadata> for SupportMetadataResponse {
    fn from(metadata: &SupportMetadata) -> Self {
        Self {
            user_id: metadata.user_id.to_string(),
            display_name: metadata.display_name.clone(),
            email_verified: metadata.email_verified,
            accounts: metadata
                .accounts
                .iter()
                .map(|account| SupportAccountDto {
                    id: account.id.to_string(),
                    label: account.label.clone(),
                    source_type: account.source_type.clone(),
                    sync_status: account.sync_status.clone(),
                    last_synced_at: account.last_synced_at,
                })
                .collect(),
            portfolios: metadata
                .portfolios
                .iter()
                .map(|portfolio| SupportPortfolioDto {
                    id: portfolio.id.to_string(),
                    label: portfolio.label.clone(),
                })
                .collect(),
        }
    }
}

/// The parsed form of a verified gate token, exposed for the Auth hook adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedGate {
    /// The jurisdiction the token covers.
    pub jurisdiction: String,
    /// When the token expires.
    pub expires_at: DateTime<Utc>,
}

impl From<GateGrant> for VerifiedGate {
    fn from(grant: GateGrant) -> Self {
        Self {
            jurisdiction: grant.jurisdiction,
            expires_at: grant.expires_at,
        }
    }
}

/// `POST /v1/internal/jurisdiction/check` body (C8).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JurisdictionCheckRequest {
    /// The self-declared ISO 3166-1 alpha-2 code.
    pub country_code: String,
    /// The address the trusted caller observed, never a client-supplied header.
    pub ip: String,
}

/// The check response (C8).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JurisdictionCheckResponse {
    /// Whether the check may proceed.
    pub allowed: bool,
    /// `self_declared`, `ip`, or `both`; absent when allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restricted_by: Option<String>,
    /// The list version the decision cites.
    pub list_version: String,
    /// The evidence record the decision wrote.
    pub evidence_id: String,
}

impl JurisdictionCheckResponse {
    /// Maps the application result at the boundary.
    #[must_use]
    pub fn from_result(result: &CheckResult) -> Self {
        Self {
            allowed: result.decision().is_allowed(),
            restricted_by: result
                .decision()
                .restricted_by()
                .map(|source| source.as_str().to_string()),
            list_version: result.decision().list_version().to_string(),
            evidence_id: result.evidence_id().to_string(),
        }
    }
}

/// One entry of a publish request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JurisdictionEntryRequest {
    /// The ISO 3166-1 alpha-2 code.
    pub country_code: String,
    /// `allowed` or `restricted`.
    pub status: String,
    /// Who maintains the entry.
    pub source: String,
}

/// `POST /v1/internal/jurisdiction/publish` body (W3).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JurisdictionPublishRequest {
    /// The monotonic version label.
    pub version: String,
    /// The complete set of country rules.
    pub entries: Vec<JurisdictionEntryRequest>,
    /// An optional caller-computed digest to verify.
    #[serde(default)]
    pub checksum: Option<String>,
}

impl JurisdictionPublishRequest {
    /// Maps the wire form to the application request.
    #[must_use]
    pub fn into_application(self) -> PublishRequest {
        PublishRequest {
            version: self.version,
            entries: self
                .entries
                .into_iter()
                .map(|entry| PublishEntry {
                    country_code: entry.country_code,
                    status: entry.status,
                    source: entry.source,
                })
                .collect(),
            expected_checksum: self.checksum,
        }
    }
}

/// The publish response (W3).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JurisdictionPublishResponse {
    /// The published version label.
    pub version: String,
    /// How many countries the version names.
    pub entry_count: usize,
    /// The digest over the ordered entries.
    pub checksum: String,
}

impl JurisdictionPublishResponse {
    /// Maps the application result at the boundary.
    #[must_use]
    pub fn from_published(list: &PublishedList) -> Self {
        Self {
            version: list.version().to_string(),
            entry_count: list.entry_count(),
            checksum: list.checksum().to_string(),
        }
    }
}

/// `GET /v1/me/account-status` (D0) response: the caller's own status and
/// nothing else.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatusReadResponse {
    /// The lifecycle state.
    pub status: String,
}

impl From<UserStatus> for AccountStatusReadResponse {
    fn from(status: UserStatus) -> Self {
        Self {
            status: status.as_str().to_string(),
        }
    }
}

/// `GET /v1/me/profile` and `PATCH /v1/me/profile` (P1/P2) response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    /// The internal user identifier.
    pub user_id: String,
    /// The name the product shows.
    pub display_name: Option<String>,
    /// Whether the name came from the user rather than the provider.
    pub display_name_overridden: bool,
    /// The name the provider sent in this request's claim.
    pub provider_display_name: Option<String>,
    /// The stored address, read-only on this surface.
    pub email: Option<String>,
    /// Whether the provider has verified the address.
    pub email_verified: bool,
    /// The lifecycle state.
    pub status: String,
}

impl From<&ProfileView> for ProfileResponse {
    fn from(view: &ProfileView) -> Self {
        Self {
            user_id: view.user_id().to_string(),
            display_name: view.display_name().map(ToString::to_string),
            display_name_overridden: view.display_name_overridden(),
            provider_display_name: view.provider_display_name().map(ToString::to_string),
            email: view.email().map(ToString::to_string),
            email_verified: view.email_verified(),
            status: view.status().as_str().to_string(),
        }
    }
}

/// `PATCH /v1/me/profile` (P2) body: exactly one of the two actions.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdateRequest {
    /// The user-supplied display name (W1).
    #[serde(default)]
    pub display_name: Option<String>,
    /// Adopt the provider-supplied name again (W2); must be `true` when present.
    #[serde(default)]
    pub use_provider_name: Option<bool>,
}

/// Why a `PATCH /v1/me/profile` body was refused before any lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileAction {
    /// Save the supplied display name.
    DisplayName(String),
    /// Adopt the provider-supplied name.
    UseProviderName,
}

impl ProfileUpdateRequest {
    /// Resolves the body to exactly one action.
    ///
    /// # Errors
    /// Returns the field problems for both-or-neither and for
    /// `useProviderName: false`, which asks for nothing.
    pub fn action(self) -> Result<ProfileAction, Vec<FieldError>> {
        match (self.display_name, self.use_provider_name) {
            (Some(name), None) => Ok(ProfileAction::DisplayName(name)),
            (None, Some(true)) => Ok(ProfileAction::UseProviderName),
            (None, Some(false)) => Err(vec![FieldError::new(
                "useProviderName",
                "must be true when supplied",
            )]),
            _ => Err(vec![FieldError::new(
                "request",
                "exactly one of displayName or useProviderName must be supplied",
            )]),
        }
    }
}

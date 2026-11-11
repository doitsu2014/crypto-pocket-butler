//! The internal `User` record and its lifecycle (BR1.1–BR1.5).
//!
//! Supabase Auth owns credentials, factors, and tokens. This record holds only
//! the fields Identity's workflows need (NFR13.1), and the Supabase subject is
//! stored encrypted in `user_personal`, never here.

use chrono::{DateTime, Utc};

use crate::claims::{NextStep, Role, VerifiedClaims};
use crate::error::DomainError;
use crate::ids::UserId;
use crate::preferences::{BaseCurrency, Locale, Preferences, TimeZone};

/// The internal lifecycle state.
///
/// `pending` is the transient provisioning state; the four public states are
/// `active`, `disabled`, `pending_deletion`, and `deleted` (BR1.7, FR17.1).
/// Every transition after provisioning belongs to the account-lifecycle unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserStatus {
    /// The email or the required MFA is not complete; portfolio data is blocked.
    Pending,
    /// Verified and allowed to use the account according to its role.
    Active,
    /// An administrator disabled the account; history is kept.
    Disabled,
    /// The user asked for deletion and the recovery window is open.
    PendingDeletion,
    /// The account was deleted and anonymised; terminal.
    Deleted,
}

impl UserStatus {
    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            "pending_deletion" => Ok(Self::PendingDeletion),
            "deleted" => Ok(Self::Deleted),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be pending, active, disabled, pending_deletion, or deleted"
                    .to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::PendingDeletion => "pending_deletion",
            Self::Deleted => "deleted",
        }
    }

    /// Whether the state denies authenticated use (BR1.7).
    #[must_use]
    pub fn denies_use(self) -> bool {
        matches!(self, Self::Disabled | Self::PendingDeletion | Self::Deleted)
    }
}

/// The longest email address the record stores.
pub const MAX_EMAIL_LEN: usize = 320;
/// The longest display name the record stores.
pub const MAX_DISPLAY_NAME_LEN: usize = 200;

/// One internal user record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    role: Role,
    status: UserStatus,
    display_name: Option<String>,
    display_name_overridden: bool,
    email: Option<String>,
    email_overridden: bool,
    email_verified: bool,
    locale: Locale,
    base_currency: BaseCurrency,
    timezone: TimeZone,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    anonymised_at: Option<DateTime<Utc>>,
}

impl User {
    /// Creates the record for a first authenticated request (WF1 step 3).
    ///
    /// The initial status follows the provider's verified-email fact: a
    /// confirmed address (and, for an exchange-credential holder, AAL2) starts
    /// `active`; anything short of that starts `pending` and the next verified
    /// request promotes it through the guarded transition.
    #[must_use]
    pub fn provision(id: UserId, claims: &VerifiedClaims, now: DateTime<Utc>) -> Self {
        let preferences = Preferences::default();
        let email = normalized_email(claims.email.as_deref());
        let email_verified = claims.email_confirmed && email.is_some();
        Self {
            id,
            role: claims.role,
            status: if claims.can_activate() && email_verified {
                UserStatus::Active
            } else {
                UserStatus::Pending
            },
            display_name: normalized_display_name(claims.display_name.as_deref()),
            display_name_overridden: false,
            email,
            email_overridden: false,
            email_verified,
            locale: preferences.locale,
            base_currency: preferences.base_currency,
            timezone: preferences.timezone,
            created_at: now,
            updated_at: now,
            anonymised_at: None,
        }
    }

    /// Rebuilds a record from stored columns.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: UserId,
        role: Role,
        status: UserStatus,
        display_name: Option<String>,
        display_name_overridden: bool,
        email: Option<String>,
        email_overridden: bool,
        email_verified: bool,
        locale: Locale,
        base_currency: BaseCurrency,
        timezone: TimeZone,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        anonymised_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            role,
            status,
            display_name,
            display_name_overridden,
            email,
            email_overridden,
            email_verified,
            locale,
            base_currency,
            timezone,
            created_at,
            updated_at,
            anonymised_at,
        }
    }

    /// Refreshes the role and the provider-derived profile fields (BR1.4).
    ///
    /// A field the user overrode is never overwritten by a claim. The guarded
    /// `pending` → `active` promotion is the interim path until the
    /// account-lifecycle unit owns every transition.
    ///
    /// # Errors
    /// Returns [`DomainError::Denied`] for a disabled, pending-deletion, or
    /// deleted account.
    pub fn apply_claims(
        &mut self,
        claims: &VerifiedClaims,
        now: DateTime<Utc>,
    ) -> Result<NextStep, DomainError> {
        match self.status {
            UserStatus::Deleted => {
                return Err(DomainError::Denied {
                    reason: "this account has been deleted".to_string(),
                });
            }
            UserStatus::Disabled => {
                return Err(DomainError::Denied {
                    reason: "this account is disabled".to_string(),
                });
            }
            UserStatus::PendingDeletion => {
                return Err(DomainError::Denied {
                    reason: "this account is pending deletion".to_string(),
                });
            }
            UserStatus::Pending | UserStatus::Active => {}
        }

        let mut changed = false;
        if self.role != claims.role {
            self.role = claims.role;
            changed = true;
        }
        if !self.display_name_overridden {
            if let Some(display_name) = normalized_display_name(claims.display_name.as_deref()) {
                if self.display_name.as_deref() != Some(display_name.as_str()) {
                    self.display_name = Some(display_name);
                    changed = true;
                }
            }
        }
        if !self.email_overridden {
            if let Some(email) = normalized_email(claims.email.as_deref()) {
                if self.email.as_deref() != Some(email.as_str()) {
                    self.email = Some(email);
                    changed = true;
                }
            }
            let verified = claims.email_confirmed && self.email.is_some();
            if self.email_verified != verified {
                self.email_verified = verified;
                changed = true;
            }
        }
        if self.status == UserStatus::Pending && claims.can_activate() {
            // U5 owns every status change: the interim promotion runs through
            // the same guarded edge the account-lifecycle unit exposes.
            self.transition(UserStatus::Active, now)?;
            changed = true;
        }
        if changed {
            self.updated_at = now;
        }
        Ok(claims.next_step())
    }

    /// Moves the account along one permitted edge of the status machine
    /// (BR5.1). Every status write in the service goes through this method.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] for an edge outside the
    /// machine; the record is unchanged.
    pub fn transition(&mut self, to: UserStatus, now: DateTime<Utc>) -> Result<(), DomainError> {
        crate::lifecycle::require_edge(self.status, to)?;
        self.status = to;
        self.updated_at = now;
        Ok(())
    }

    /// Applies a user-supplied profile change and marks the fields overridden.
    ///
    /// An overridden address is not provider-verified, so changing the email
    /// clears the verification fact until the provider proves the new address.
    ///
    /// # Errors
    /// Returns [`DomainError`] naming the offending field; the record is
    /// unchanged when any supplied value is refused.
    pub fn update_profile(
        &mut self,
        display_name: Option<String>,
        email: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let display_name = display_name
            .map(|value| validate_display_name(&value))
            .transpose()?;
        let email = email.map(|value| validate_email(&value)).transpose()?;
        if display_name.is_none() && email.is_none() {
            return Err(DomainError::Invalid {
                field: "profile",
                reason: "at least one profile field must be supplied".to_string(),
            });
        }
        if let Some(display_name) = display_name {
            self.display_name = Some(display_name);
            self.display_name_overridden = true;
        }
        if let Some(email) = email {
            if self.email.as_deref() != Some(email.as_str()) {
                self.email_verified = false;
            }
            self.email = Some(email);
            self.email_overridden = true;
        }
        self.updated_at = now;
        Ok(())
    }

    /// Applies a validated preference change (BR3.3).
    pub fn update_preferences(&mut self, preferences: Preferences, now: DateTime<Utc>) {
        self.locale = preferences.locale;
        self.base_currency = preferences.base_currency;
        self.timezone = preferences.timezone;
        self.updated_at = now;
    }

    /// Anonymises the personal fields on deletion (BR6.2).
    pub fn anonymise(&mut self, now: DateTime<Utc>) {
        let preferences = Preferences::default();
        self.locale = preferences.locale;
        self.base_currency = preferences.base_currency;
        self.timezone = preferences.timezone;
        self.role = Role::User;
        self.status = UserStatus::Deleted;
        self.display_name = None;
        self.display_name_overridden = false;
        self.email = None;
        self.email_overridden = false;
        self.email_verified = false;
        self.anonymised_at = Some(now);
        self.updated_at = now;
    }

    /// The display name shown in the product.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Whether the display name came from the user rather than the provider.
    #[must_use]
    pub fn display_name_overridden(&self) -> bool {
        self.display_name_overridden
    }

    /// The stored email address, whatever its verification state.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Whether the stored address came from the user rather than the provider.
    #[must_use]
    pub fn email_overridden(&self) -> bool {
        self.email_overridden
    }

    /// Whether the stored address is confirmed.
    #[must_use]
    pub fn email_verified(&self) -> bool {
        self.email_verified
    }

    /// The address C3 may hand to a service caller: only a confirmed one.
    #[must_use]
    pub fn verified_email(&self) -> Option<&str> {
        if self.email_verified {
            self.email.as_deref()
        } else {
            None
        }
    }

    /// The internal identifier.
    #[must_use]
    pub fn id(&self) -> UserId {
        self.id
    }

    /// The capability role.
    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> UserStatus {
        self.status
    }

    /// Whether the account is deleted.
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.status == UserStatus::Deleted
    }

    /// Whether the account is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    /// The UI language.
    #[must_use]
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// The valuation base currency.
    #[must_use]
    pub fn base_currency(&self) -> BaseCurrency {
        self.base_currency.clone()
    }

    /// The rendering timezone.
    #[must_use]
    pub fn timezone(&self) -> TimeZone {
        self.timezone.clone()
    }

    /// When the record was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the record last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// When the personal fields were anonymised.
    #[must_use]
    pub fn anonymised_at(&self) -> Option<DateTime<Utc>> {
        self.anonymised_at
    }
}

/// Trims a claim-supplied address and drops an empty or over-long one.
fn normalized_email(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().count() <= MAX_EMAIL_LEN)
        .map(ToString::to_string)
}

/// Trims a claim-supplied display name and drops an empty or over-long one.
fn normalized_display_name(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().count() <= MAX_DISPLAY_NAME_LEN)
        .map(ToString::to_string)
}

/// Validates a user-supplied display name, returning its trimmed form.
fn validate_display_name(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required {
            field: "displayName",
        });
    }
    if value.chars().count() > MAX_DISPLAY_NAME_LEN {
        return Err(DomainError::TooLarge {
            field: "displayName",
            limit: "200 characters",
        });
    }
    Ok(value.to_string())
}

/// Validates a user-supplied email address in RFC 5322 shape, returning its
/// trimmed form. The check is deliberately conservative: one `@`, a non-empty
/// local part, a dotted domain, and no whitespace.
fn validate_email(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "email" });
    }
    if value.chars().count() > MAX_EMAIL_LEN || !is_email_shaped(value) {
        return Err(DomainError::Invalid {
            field: "email",
            reason: "must be an RFC 5322 address".to_string(),
        });
    }
    Ok(value.to_string())
}

/// Whether a value is shaped like a mail address.
#[must_use]
pub fn is_email_shaped(value: &str) -> bool {
    if value.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
}

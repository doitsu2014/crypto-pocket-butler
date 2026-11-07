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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserStatus {
    /// The email or the required MFA is not complete; portfolio data is blocked.
    Pending,
    /// Verified and allowed to use the account according to its role.
    Active,
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
            "deleted" => Ok(Self::Deleted),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be pending, active, or deleted".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Deleted => "deleted",
        }
    }
}

/// One internal user record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    role: Role,
    status: UserStatus,
    locale: Locale,
    base_currency: BaseCurrency,
    timezone: TimeZone,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    anonymised_at: Option<DateTime<Utc>>,
}

impl User {
    /// Creates the record for a first authenticated request (WF1 step 3).
    #[must_use]
    pub fn provision(id: UserId, claims: &VerifiedClaims, now: DateTime<Utc>) -> Self {
        let preferences = Preferences::default();
        Self {
            id,
            role: claims.role,
            status: UserStatus::Pending,
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
            locale,
            base_currency,
            timezone,
            created_at,
            updated_at,
            anonymised_at,
        }
    }

    /// Refreshes the role from the signed token and applies BR1.5.
    ///
    /// # Errors
    /// Returns [`DomainError::Denied`] for a deleted account.
    pub fn apply_claims(
        &mut self,
        claims: &VerifiedClaims,
        now: DateTime<Utc>,
    ) -> Result<NextStep, DomainError> {
        if self.status == UserStatus::Deleted {
            return Err(DomainError::Denied {
                reason: "this account has been deleted".to_string(),
            });
        }

        let mut changed = false;
        if self.role != claims.role {
            self.role = claims.role;
            changed = true;
        }
        if self.status == UserStatus::Pending && claims.can_activate() {
            self.status = UserStatus::Active;
            changed = true;
        }
        if changed {
            self.updated_at = now;
        }
        Ok(claims.next_step())
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
        self.anonymised_at = Some(now);
        self.updated_at = now;
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

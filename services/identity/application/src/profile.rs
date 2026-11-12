//! U7 — the signed-in person's profile and account-status reads (BR1.4, BR3.1–BR3.2).
//!
//! `ProfileService` owns the caller's own profile view (P1), the display-name
//! save (W1), and the provider-name adoption (W2). Both writes append one
//! `identity.user.profile.updated` audit event naming the field and the override
//! state; both reads append nothing. D0's status read answers every status
//! without the `UserStatus::denies_use` predicate, because the blocked and
//! restore surfaces must still learn where the account stands.
//!
//! The owner is always the caller resolved from the validated token — no method
//! here accepts a user id from the request.

use std::sync::Arc;

use identity_domain::{AuditOutcome, User, UserId, UserStatus};

use crate::common::{AppError, Caller};
use crate::ports::{Clock, UserRepository};
use crate::trail::{context, Trail};

/// The audit action both profile writes append.
const PROFILE_UPDATED: &str = "identity.user.profile.updated";

/// The caller's own profile as the settings surface renders it (P1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileView {
    user_id: UserId,
    display_name: Option<String>,
    display_name_overridden: bool,
    provider_display_name: Option<String>,
    email: Option<String>,
    email_verified: bool,
    status: UserStatus,
}

impl ProfileView {
    /// Builds the view from the stored record and the forwarded claim.
    ///
    /// The provider display name travels in the claim on every request, so it is
    /// never stored a second time; it stays offered after an override so the
    /// person can adopt it again (W2).
    #[must_use]
    pub fn new(user: &User, provider_display_name: Option<&str>) -> Self {
        Self {
            user_id: user.id(),
            display_name: user.display_name().map(ToString::to_string),
            display_name_overridden: user.display_name_overridden(),
            provider_display_name: provider_display_name.map(ToString::to_string),
            email: user.email().map(ToString::to_string),
            email_verified: user.email_verified(),
            status: user.status(),
        }
    }

    /// The internal user identifier.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The name the product shows.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Whether that name came from the user rather than the provider.
    #[must_use]
    pub fn display_name_overridden(&self) -> bool {
        self.display_name_overridden
    }

    /// The name the provider sent in this request's claim.
    #[must_use]
    pub fn provider_display_name(&self) -> Option<&str> {
        self.provider_display_name.as_deref()
    }

    /// The stored address, read-only on this surface.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Whether the provider has verified the stored address.
    #[must_use]
    pub fn email_verified(&self) -> bool {
        self.email_verified
    }

    /// The account's lifecycle state.
    #[must_use]
    pub fn status(&self) -> UserStatus {
        self.status
    }
}

/// The profile read and write use cases.
pub struct ProfileService {
    users: Arc<dyn UserRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl ProfileService {
    /// Creates the use case.
    #[must_use]
    pub fn new(users: Arc<dyn UserRepository>, trail: Arc<Trail>, clock: Arc<dyn Clock>) -> Self {
        Self {
            users,
            trail,
            clock,
        }
    }

    /// P1: the caller's own profile. A read, so no audit event is written.
    #[must_use]
    pub fn view(&self, caller: &Caller) -> ProfileView {
        ProfileView::new(&caller.user, caller.claims.display_name.as_deref())
    }

    /// D0: the caller's own lifecycle status, answered for every status.
    ///
    /// The blocked, restore, and deleted surfaces need the status of an account
    /// the ordinary `load_caller` path would deny, so this read never consults
    /// `UserStatus::denies_use`. It is a read: no audit event, no write.
    #[must_use]
    pub fn account_status(&self, caller: &Caller) -> UserStatus {
        caller.user.status()
    }

    /// W1: stores a user-supplied display name and marks it overridden.
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] naming `displayName` for a blank or
    /// over-long value, and [`AppError::Unavailable`] when the write fails. A
    /// refusal leaves the record and the audit trail unchanged.
    pub async fn update_display_name(
        &self,
        caller: &Caller,
        display_name: &str,
    ) -> Result<ProfileView, AppError> {
        let mut user = caller.user.clone();
        user.update_profile(Some(display_name.to_string()), None, self.clock.now())
            .map_err(AppError::from_domain)?;
        self.users
            .update(&user)
            .await
            .map_err(AppError::from_repository)?;
        self.record(caller, user.id(), "true").await?;
        Ok(ProfileView::new(
            &user,
            caller.claims.display_name.as_deref(),
        ))
    }

    /// W2: adopts the provider-supplied name again and clears the override.
    ///
    /// # Errors
    /// Returns [`AppError::Conflict`] when the forwarded claim carries no usable
    /// display name — clearing the flag alone would not change what the person
    /// sees, so the action carries no effect — and [`AppError::Unavailable`]
    /// when the write fails.
    pub async fn adopt_provider_name(&self, caller: &Caller) -> Result<ProfileView, AppError> {
        let mut user = caller.user.clone();
        let provider_name = caller.claims.display_name.as_deref();
        if !user.adopt_provider_name(provider_name, self.clock.now()) {
            return Err(AppError::Conflict(
                "the provider supplied no display name to adopt".to_string(),
            ));
        }
        self.users
            .update(&user)
            .await
            .map_err(AppError::from_repository)?;
        self.record(caller, user.id(), "false").await?;
        Ok(ProfileView::new(&user, provider_name))
    }

    async fn record(
        &self,
        caller: &Caller,
        user_id: UserId,
        overridden: &'static str,
    ) -> Result<(), AppError> {
        self.trail
            .record(
                Some(caller.user_id()),
                PROFILE_UPDATED,
                user_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("field", "displayName"), ("overridden", overridden)]),
            )
            .await
    }
}

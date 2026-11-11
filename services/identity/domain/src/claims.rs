//! The facts Identity derives from a Supabase JWT it validated itself.
//!
//! Credentials, MFA factors, and tokens stay in Supabase Auth. Identity reads the
//! role, the verified-email fact, the assurance level, and the exchange-credential
//! fact from the signed token (BR1.2, BR1.5, BR2.1).

/// A capability role. It comes only from `app_metadata.role`; anything else is a
/// plain user (BR1.2).
///
/// `System` exists only for a scheduled job's caller identity: no claim, header,
/// body, or stored role can produce it (BR2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// A regular user; the authorisation vocabulary calls this role `owner`.
    User,
    /// An administrator.
    Admin,
    /// A support operator.
    Support,
    /// A scheduled job acting for no user.
    System,
}

impl Role {
    /// Reads the role claim, treating a missing or unknown value as `user`.
    ///
    /// This is the one place a claim becomes a role (BR2.2): only `admin` and
    /// `support` are recognised, and no value here can ever produce `system`.
    #[must_use]
    pub fn from_claim(raw: Option<&str>) -> Self {
        match raw.map(str::trim) {
            Some("admin") => Self::Admin,
            Some("support") => Self::Support,
            _ => Self::User,
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
            Self::Support => "support",
            Self::System => "system",
        }
    }

    /// The value written to the provider's `app_metadata.role` claim (BR2.2).
    ///
    /// The provider vocabulary names the regular user `owner`; any value other
    /// than `admin` or `support` reads back as owner, so writing `owner` is
    /// stable. `system` is never written: it exists only for jobs.
    #[must_use]
    pub fn provider_value(self) -> &'static str {
        match self {
            Self::User => "owner",
            Self::Admin => "admin",
            Self::Support => "support",
            Self::System => "system",
        }
    }

    /// Whether the role carries the admin capability.
    #[must_use]
    pub fn is_admin(self) -> bool {
        self == Self::Admin
    }

    /// Whether the role carries the support capability.
    #[must_use]
    pub fn is_support(self) -> bool {
        self == Self::Support
    }

    /// Whether this is the job-only system role.
    #[must_use]
    pub fn is_system(self) -> bool {
        self == Self::System
    }

    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`crate::error::DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, crate::error::DomainError> {
        match raw.trim() {
            "user" => Ok(Self::User),
            "admin" => Ok(Self::Admin),
            "support" => Ok(Self::Support),
            _ => Err(crate::error::DomainError::Invalid {
                field: "role",
                reason: "must be user, admin, or support".to_string(),
            }),
        }
    }
}

/// The authentication assurance level carried by the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssuranceLevel {
    /// A single factor.
    Aal1,
    /// Two factors.
    Aal2,
}

impl AssuranceLevel {
    /// Reads the `aal` claim; a missing or unknown value is AAL1.
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim) {
            Some("aal2") => Self::Aal2,
            _ => Self::Aal1,
        }
    }
}

/// What the caller must do next before portfolio data is reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NextStep {
    /// Nothing further is required.
    None,
    /// Confirm the email address.
    VerifyEmail,
    /// Enrol a second factor.
    EnrolMfa,
}

impl NextStep {
    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::VerifyEmail => "verify_email",
            Self::EnrolMfa => "enrol_mfa",
        }
    }
}

/// The validated facts from one Supabase JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedClaims {
    /// The Supabase Auth subject.
    pub subject: String,
    /// `app_metadata.role`.
    pub role: Role,
    /// Whether the email is confirmed (BR1.5).
    pub email_confirmed: bool,
    /// The email address carried by the token, when Supabase includes it.
    ///
    /// Identity stores it on the user record so C3 can answer a service caller
    /// with the verified address; Notification never stores it (NFR9.3).
    pub email: Option<String>,
    /// The display name the identity provider supplies, when it supplies one.
    pub display_name: Option<String>,
    /// The `aal` claim.
    pub assurance: AssuranceLevel,
    /// `app_metadata.has_exchange_credentials` (BR2.1).
    pub has_exchange_credentials: bool,
    /// The Supabase Auth session identifier, used for the session view.
    pub session_id: Option<String>,
    /// A short client label derived from the user agent.
    pub user_agent: Option<String>,
    /// The gateway's coarse city/country geolocation, never the full IP.
    pub location: Option<String>,
    /// The self-declared ISO 3166-1 alpha-2 country supplied by the trusted
    /// caller; used only by the jurisdiction gate at first provisioning (C8).
    pub country_code: Option<String>,
    /// The client address observed by the trusted caller, never a client-written
    /// header; used only by the jurisdiction gate at first provisioning (C8).
    pub client_ip: Option<String>,
}

impl VerifiedClaims {
    /// The next step required by BR1.5 and BR2.1.
    #[must_use]
    pub fn next_step(&self) -> NextStep {
        if !self.email_confirmed {
            NextStep::VerifyEmail
        } else if self.has_exchange_credentials && self.assurance != AssuranceLevel::Aal2 {
            NextStep::EnrolMfa
        } else {
            NextStep::None
        }
    }

    /// Whether this token satisfies the conditions for `active` (BR1.5).
    #[must_use]
    pub fn can_activate(&self) -> bool {
        self.next_step() == NextStep::None
    }
}

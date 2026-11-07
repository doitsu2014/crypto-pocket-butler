//! The facts Identity derives from a Supabase JWT it validated itself.
//!
//! Credentials, MFA factors, and tokens stay in Supabase Auth. Identity reads the
//! role, the verified-email fact, the assurance level, and the exchange-credential
//! fact from the signed token (BR1.2, BR1.5, BR2.1).

/// A capability role. It comes only from `app_metadata.role`; anything else is a
/// plain user (BR1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// A regular user.
    User,
    /// An administrator.
    Admin,
    /// A support operator.
    Support,
}

impl Role {
    /// Reads the role claim, treating a missing or unknown value as `user`.
    #[must_use]
    pub fn from_claim(raw: Option<&str>) -> Self {
        match raw.map(str::trim) {
            Some("admin") => Self::Admin,
            Some("support") => Self::Support,
            Some("user") | Some("") | None => Self::User,
            Some(_) => Self::User,
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
            Self::Support => "support",
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

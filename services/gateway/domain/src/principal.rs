//! The authenticated caller and its role.

/// The caller's role, taken from `app_metadata.role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A regular user.
    User,
    /// A registry or platform administrator.
    Admin,
    /// A support operator.
    Support,
}

/// The authenticated caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// The Supabase Auth subject.
    pub subject: String,
    /// The caller's role.
    pub role: Role,
}

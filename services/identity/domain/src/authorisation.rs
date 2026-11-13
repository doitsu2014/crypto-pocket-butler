//! The authorisation decision's value objects (BR2.1–BR2.9, BR2.14).
//!
//! The decision is total: one catalogue lookup, one status check, one role
//! comparison, and one ownership assertion produce an allow or a deny. Nothing
//! here reads a store or a framework type.
//!
//! The role vocabulary reuses [`crate::claims::Role`] so a claim, a stored row,
//! and an authorisation decision never disagree about what a role is.

pub use crate::claims::Role;

use crate::ids::UserId;

/// The class the catalogue assigns to an operation. The class is fixed by the
/// catalogue, never by the caller, so a request cannot promote itself (BR2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationClass {
    /// One user-owned row: role, status, support denial, and ownership apply.
    OwnerScoped,
    /// A list, search, or export: the decision returns the owner filter.
    Collection,
    /// Administrative management: the role alone decides (BR2.3, BR2.4).
    Administrative,
    /// The one operation a `pending` account may reach (BR2.7).
    Confirmation,
    /// The support console's metadata read, decided by the u3 grant check.
    SupportMetadata,
    /// A name no catalogue entry holds (BR2.1).
    Undeclared,
}

impl OperationClass {
    /// The contract vocabulary.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OwnerScoped => "owner-scoped",
            Self::Collection => "collection",
            Self::Administrative => "administrative",
            Self::Confirmation => "confirmation",
            Self::SupportMetadata => "support-metadata",
            Self::Undeclared => "undeclared",
        }
    }

    /// Whether the class is decided by the role rather than by ownership.
    #[must_use]
    pub fn is_administrative(self) -> bool {
        self == Self::Administrative
    }
}

/// One named operation and the class the catalogue fixed for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    name: String,
    class: OperationClass,
}

impl Operation {
    /// Builds an operation. Only the catalogue should construct these, so the
    /// class always comes from a catalogue entry.
    #[must_use]
    pub fn new(name: impl Into<String>, class: OperationClass) -> Self {
        Self {
            name: name.into(),
            class,
        }
    }

    /// The operation's catalogue name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The catalogue class.
    #[must_use]
    pub fn class(&self) -> OperationClass {
        self.class
    }
}

/// Whether the caller asserts one row's owner or a collection obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceScope {
    /// One row; `resourceOwnerId` must be present and match the caller.
    Resource,
    /// A list, search, or export; the caller carries no owner id.
    Collection,
}

/// The ownership assertion a caller supplies for what it is about to touch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceOwnership {
    resource_kind: String,
    scope: ResourceScope,
    resource_owner_id: Option<UserId>,
}

impl ResourceOwnership {
    /// One row and, when the service loaded one, its owner.
    #[must_use]
    pub fn row(resource_kind: impl Into<String>, resource_owner_id: Option<UserId>) -> Self {
        Self {
            resource_kind: resource_kind.into(),
            scope: ResourceScope::Resource,
            resource_owner_id,
        }
    }

    /// A collection obligation: the owning service applies the returned filter.
    #[must_use]
    pub fn collection(resource_kind: impl Into<String>) -> Self {
        Self {
            resource_kind: resource_kind.into(),
            scope: ResourceScope::Collection,
            resource_owner_id: None,
        }
    }

    /// The resource's domain name (account, portfolio, transaction, …).
    #[must_use]
    pub fn resource_kind(&self) -> &str {
        &self.resource_kind
    }

    /// Whether the assertion names one row or a collection.
    #[must_use]
    pub fn scope(&self) -> ResourceScope {
        self.scope
    }

    /// The row's owner, when the service supplied one.
    #[must_use]
    pub fn resource_owner_id(&self) -> Option<UserId> {
        self.resource_owner_id
    }
}

/// The caller a decision is made for: a user, or the system acting for no user.
///
/// A user identity is only ever built from a token that passed signature,
/// issuer, audience, and expiry validation; a system identity is never an owner
/// and never an admin (BR2.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallerIdentity {
    /// A validated user token, resolved to its internal record.
    User {
        /// The provider subject from the validated token.
        subject: String,
        /// The internal user id resolved for that subject.
        user_id: UserId,
        /// The role from `app_metadata.role` only (BR2.2).
        role: Role,
    },
    /// A job acting for no user, bound to the one owner it iterates for.
    System {
        /// The job or service name; never a user id.
        service: String,
        /// The owner the job is iterating for. Every row it reads or writes must
        /// carry this owner, and a collection is filtered to it (BR2.8).
        owner: UserId,
    },
}

impl CallerIdentity {
    /// A user caller.
    #[must_use]
    pub fn user(subject: impl Into<String>, user_id: UserId, role: Role) -> Self {
        Self::User {
            subject: subject.into(),
            user_id,
            role,
        }
    }

    /// A system caller. It is always bound to the owner it iterates for.
    #[must_use]
    pub fn system(service: impl Into<String>, owner: UserId) -> Self {
        Self::System {
            service: service.into(),
            owner,
        }
    }

    /// The caller's role.
    #[must_use]
    pub fn role(&self) -> Role {
        match self {
            Self::User { role, .. } => *role,
            Self::System { .. } => Role::System,
        }
    }

    /// The internal user id, when the caller is a user.
    #[must_use]
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            Self::User { user_id, .. } => Some(*user_id),
            Self::System { .. } => None,
        }
    }

    /// The owner the caller may act for: a user acts for itself, and a job acts
    /// only for the owner it was bound to when it started the iteration (BR2.8).
    #[must_use]
    pub fn owner(&self) -> Option<UserId> {
        match self {
            Self::User { user_id, .. } => Some(*user_id),
            Self::System { owner, .. } => Some(*owner),
        }
    }

    /// The provider subject, when the caller is a user.
    #[must_use]
    pub fn subject(&self) -> Option<&str> {
        match self {
            Self::User { subject, .. } => Some(subject),
            Self::System { .. } => None,
        }
    }
    /// The identifier the audit record names as the actor: a user id or a
    /// service name.
    #[must_use]
    pub fn actor_label(&self) -> String {
        match self {
            Self::User { user_id, .. } => user_id.to_string(),
            Self::System { service, .. } => service.clone(),
        }
    }

    /// Whether this is the job-only system caller.
    #[must_use]
    pub fn is_system(&self) -> bool {
        matches!(self, Self::System { .. })
    }
}

/// Why a request was refused; `allowed` is the reason an allow carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DenialReason {
    /// The request was permitted.
    Allowed,
    /// The role does not permit the operation (BR2.4, BR2.14).
    NoRole,
    /// The account's status denies use (BR2.7).
    StatusDenied,
    /// The caller does not own the row (BR2.5).
    NotOwner,
    /// No catalogue entry permits the operation (BR2.1).
    NotDeclared,
    /// The refusal could not be recorded, so the request fails closed (BR2.11).
    AuditUnavailable,
}

impl DenialReason {
    /// The contract vocabulary.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::NoRole => "no_role",
            Self::StatusDenied => "status_denied",
            Self::NotOwner => "not_owner",
            Self::NotDeclared => "not_declared",
            Self::AuditUnavailable => "audit_unavailable",
        }
    }

    /// The status the caller answers with. `not_owner` and a nonexistent id are
    /// both 404 and reveal nothing (BR2.6); `audit_unavailable` is the only 503.
    #[must_use]
    pub fn http_status(self) -> u16 {
        match self {
            Self::Allowed => 200,
            Self::NotOwner => 404,
            Self::AuditUnavailable => 503,
            Self::NoRole | Self::StatusDenied | Self::NotDeclared => 403,
        }
    }
}

/// The outcome of one evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    allowed: bool,
    reason: DenialReason,
    http_status: u16,
    filter_owner_id: Option<UserId>,
}

impl Decision {
    /// An allow, carrying the owner filter when the operation was a collection.
    #[must_use]
    pub fn allow(filter_owner_id: Option<UserId>) -> Self {
        Self {
            allowed: true,
            reason: DenialReason::Allowed,
            http_status: DenialReason::Allowed.http_status(),
            filter_owner_id,
        }
    }

    /// A deny for one of the catalogue's reasons.
    #[must_use]
    pub fn deny(reason: DenialReason) -> Self {
        Self {
            allowed: false,
            reason,
            http_status: reason.http_status(),
            filter_owner_id: None,
        }
    }

    /// A refusal that could not be audited; the caller answers 503 (BR2.11).
    #[must_use]
    pub fn audit_unavailable() -> Self {
        Self::deny(DenialReason::AuditUnavailable)
    }

    /// Whether the request is permitted.
    #[must_use]
    pub fn allowed(&self) -> bool {
        self.allowed
    }

    /// Why.
    #[must_use]
    pub fn reason(&self) -> DenialReason {
        self.reason
    }

    /// The status the caller answers with.
    #[must_use]
    pub fn http_status(&self) -> u16 {
        self.http_status
    }

    /// The owner every returned row must carry, on an allowed collection.
    #[must_use]
    pub fn filter_owner_id(&self) -> Option<UserId> {
        self.filter_owner_id
    }
}

//! The mutable ledger subject: transactions reference it instead of the user
//! id, so anonymisation never rewrites an append-only row (NFR9.2).

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::LedgerSubjectId;

/// A mutable subject that carries the user id and the anonymisation reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerSubject {
    id: LedgerSubjectId,
    user_id: Option<String>,
    anonymised_ref: Option<String>,
    created_at: DateTime<Utc>,
}

impl LedgerSubject {
    /// Creates a subject for a user.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] when the user id is blank.
    pub fn new(
        id: LedgerSubjectId,
        user_id: impl Into<String>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let user_id = user_id.into();
        if user_id.trim().is_empty() {
            return Err(DomainError::Required { field: "userId" });
        }
        Ok(Self {
            id,
            user_id: Some(user_id.trim().to_string()),
            anonymised_ref: None,
            created_at,
        })
    }

    /// Rehydrates a stored subject.
    #[must_use]
    pub fn from_stored(
        id: LedgerSubjectId,
        user_id: Option<String>,
        anonymised_ref: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            anonymised_ref,
            created_at,
        }
    }

    /// The identifier.
    #[must_use]
    pub fn id(&self) -> LedgerSubjectId {
        self.id
    }

    /// The Supabase Auth subject, cleared once anonymised.
    #[must_use]
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// The non-reversible reference set by anonymisation.
    #[must_use]
    pub fn anonymised_ref(&self) -> Option<&str> {
        self.anonymised_ref.as_deref()
    }

    /// When the subject was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Whether anonymisation has already applied.
    #[must_use]
    pub fn is_anonymised(&self) -> bool {
        self.user_id.is_none() && self.anonymised_ref.is_some()
    }
}

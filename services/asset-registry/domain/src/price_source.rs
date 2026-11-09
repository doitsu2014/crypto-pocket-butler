//! Price sources and their priority order (BR2.1).

use std::collections::HashSet;

use crate::error::DomainError;
use crate::ids::PriceSourceId;

/// A provider that can price assets, with a configured priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSource {
    id: PriceSourceId,
    name: String,
    priority: u32,
    licence_ref: Option<String>,
    enabled: bool,
}

impl PriceSource {
    /// Creates a price source. A source with a licence reference starts enabled;
    /// a source without one starts disabled and may not be enabled for production.
    ///
    /// # Errors
    /// Returns a [`DomainError`] for an invalid identifier, blank name, or a
    /// priority below 1.
    pub fn new(
        id: impl Into<String>,
        name: &str,
        priority: u32,
        licence_ref: Option<&str>,
    ) -> Result<Self, DomainError> {
        let id = PriceSourceId::new(id)?;
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::EmptyName);
        }
        if priority == 0 {
            return Err(DomainError::InvalidPriority(priority));
        }
        let licence_ref = licence_ref
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let enabled = licence_ref.is_some();
        Ok(Self {
            id,
            name: name.to_string(),
            priority,
            licence_ref,
            enabled,
        })
    }

    /// The stable opaque identifier.
    #[must_use]
    pub fn id(&self) -> &PriceSourceId {
        &self.id
    }

    /// The provider name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Lower values are tried first.
    #[must_use]
    pub fn priority(&self) -> u32 {
        self.priority
    }

    /// The licence that permits production use, if any.
    #[must_use]
    pub fn licence_ref(&self) -> Option<&str> {
        self.licence_ref.as_deref()
    }

    /// Whether the source participates in price resolution.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Enables the source.
    ///
    /// # Errors
    /// Returns [`DomainError::UnlicensedSource`] when no licence reference is set.
    pub fn enable(&mut self) -> Result<(), DomainError> {
        if self.licence_ref.is_none() {
            return Err(DomainError::UnlicensedSource(self.name.clone()));
        }
        self.enabled = true;
        Ok(())
    }

    /// Disables the source, removing it from the resolution order.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Changes the priority.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidPriority`] for zero.
    pub fn set_priority(&mut self, priority: u32) -> Result<(), DomainError> {
        if priority == 0 {
            return Err(DomainError::InvalidPriority(priority));
        }
        self.priority = priority;
        Ok(())
    }
}

/// BR2.1: at most one enabled source may hold a given priority.
///
/// # Errors
/// Returns [`DomainError::DuplicatePriority`] naming the duplicated priority.
pub fn ensure_unique_enabled_priorities(sources: &[PriceSource]) -> Result<(), DomainError> {
    let mut seen = HashSet::new();
    for source in sources.iter().filter(|source| source.enabled()) {
        if !seen.insert(source.priority()) {
            return Err(DomainError::DuplicatePriority(source.priority()));
        }
    }
    Ok(())
}

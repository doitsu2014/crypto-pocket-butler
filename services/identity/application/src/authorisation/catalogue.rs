//! The operation catalogue: an operation name maps to its class, and a name the
//! catalogue does not hold is undeclared (BR2.1).
//!
//! The class is fixed here, never by the caller, so a request cannot promote
//! itself to an administrative operation.

use identity_domain::{Operation, OperationClass};

/// The closed operation table. It holds no state; an unknown name is
/// `Undeclared`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OperationCatalogue;

impl OperationCatalogue {
    /// Creates the catalogue.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The class the catalogue fixes for `name`.
    #[must_use]
    pub fn class_of(&self, name: &str) -> OperationClass {
        match name.trim() {
            "principal:read" | "profile:read" | "profile:write" | "resource:read"
            | "resource:write" => OperationClass::OwnerScoped,
            "collection:list" | "collection:search" | "collection:export" => {
                OperationClass::Collection
            }
            "guard:confirm" => OperationClass::Confirmation,
            "support:metadata:read" => OperationClass::SupportMetadata,
            other if other.starts_with("admin:") => OperationClass::Administrative,
            _ => OperationClass::Undeclared,
        }
    }

    /// The operation value for `name`, classed by the catalogue.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Operation {
        let name = name.trim();
        Operation::new(name, self.class_of(name))
    }

    /// Whether the catalogue declares `name`.
    #[must_use]
    pub fn is_declared(&self, name: &str) -> bool {
        self.class_of(name) != OperationClass::Undeclared
    }
}

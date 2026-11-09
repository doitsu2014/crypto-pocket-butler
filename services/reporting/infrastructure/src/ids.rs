//! The random identifier adapter.

use reporting_application::ports::IdGenerator;
use uuid::Uuid;

/// Generates version-4 UUIDs.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn next(&self) -> Uuid {
        Uuid::new_v4()
    }
}

//! The random identifier adapter.

use ledger_application::ports::IdGenerator;
use uuid::Uuid;

/// The production identifier source.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn next(&self) -> Uuid {
        Uuid::new_v4()
    }
}

/// Domain models for holdings, allocations, and snapshots
///
/// This module provides strongly-typed structs that represent the business domain,
/// eliminating the need for loose JSON (serde_json::Value) in internal code.
/// These types are serialized/deserialized only at the database boundary.
///
/// # Architecture
///
/// - **AccountHolding**: Raw holdings from accounts (quantity-only)
/// - **AllocationItem**: Enriched holdings with prices, values, and weights
/// - **SnapshotHolding**: Point-in-time holdings preserved in snapshots
///
/// # Type Safety Benefits
///
/// 1. **Compile-time validation**: Field access is checked at compile time
/// 2. **Better IDE support**: Autocomplete and type hints work correctly
/// 3. **Refactoring safety**: Changes to schema are caught by the compiler
/// 4. **Documentation**: Field meanings are explicit in the type definitions
///
/// # JSON Schema Documentation
///
/// See [`JSON_SCHEMA.md`](./JSON_SCHEMA.md) for complete JSON schema documentation
/// including examples, field descriptions, and storage locations.

pub mod holdings;
pub mod allocation;
pub mod snapshot;

pub use holdings::AccountHolding;
pub use allocation::{AllocationItem, AllocationData, UnpricedAsset};
pub use snapshot::{SnapshotHolding, SnapshotMetadata, SnapshotData};

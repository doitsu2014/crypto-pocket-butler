/// Domain models for holdings, allocations, and snapshots
///
/// This module provides strongly-typed structs that represent the business domain,
/// eliminating the need for loose JSON (serde_json::Value) in internal code.
/// These types are serialized/deserialized only at the database boundary.

pub mod holdings;
pub mod allocation;
pub mod snapshot;

pub use holdings::AccountHolding;
pub use allocation::{AllocationItem, AllocationData, UnpricedAsset};
pub use snapshot::{SnapshotHolding, SnapshotMetadata, SnapshotData};

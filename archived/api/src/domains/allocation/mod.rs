/// Allocation Domain
///
/// Bounded Context: Portfolio allocation computation and snapshot management.
///
/// This module defines the core Allocation domain with:
/// - `AllocationItem` entity (single asset in a computed allocation)
/// - `SnapshotHolding` entity (point-in-time holding for a snapshot)
/// - `SnapshotMetadata` entity (context about a snapshot)
/// - `AllocationData` value object (complete allocation)
/// - `SnapshotData` value object (complete snapshot)
/// - `UnpricedAsset` value object (reference to an asset lacking pricing)
/// - `AllocationRepository` trait (persistence interface)
/// - `recalculate_weights` domain service

pub mod entities;
pub mod repository;
pub mod service;
pub mod value_objects;

pub use entities::{AllocationItem, SnapshotHolding, SnapshotMetadata};
pub use repository::AllocationRepository;
pub use value_objects::{AllocationData, SnapshotData, UnpricedAsset};

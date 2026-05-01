/// AssetRepository — re-exported from the domain layer.
///
/// Application-layer code (use cases, services) should import this trait
/// via `crate::application::repositories::asset_repository::AssetRepository`
/// rather than importing from the domain directly.

pub use crate::domains::asset::repository::AssetRepository;

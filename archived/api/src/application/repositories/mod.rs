/// Application-layer repository trait definitions.
///
/// These re-export the repository traits defined in the domain layer, making
/// them accessible from the application layer without requiring consumers to
/// reach into the domain modules directly.
///
/// Infrastructure implementations live in `crate::infrastructure::persistence`.

pub mod account_repository;
pub mod asset_repository;
pub mod portfolio_repository;

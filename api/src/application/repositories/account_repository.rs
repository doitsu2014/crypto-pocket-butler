/// AccountRepository — re-exported from the domain layer.
///
/// Application-layer code (use cases, services) should import this trait
/// via `crate::application::repositories::account_repository::AccountRepository`
/// rather than importing from the domain directly.

pub use crate::domains::account::repository::AccountRepository;

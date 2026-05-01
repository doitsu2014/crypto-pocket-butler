/// PortfolioRepository — re-exported from the domain layer.
///
/// Application-layer code (use cases, services) should import this trait
/// via `crate::application::repositories::portfolio_repository::PortfolioRepository`
/// rather than importing from the domain directly.

pub use crate::domains::portfolio::repository::PortfolioRepository;

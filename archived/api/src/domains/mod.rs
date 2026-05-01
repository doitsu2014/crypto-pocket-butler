/// Bounded contexts following Domain-Driven Design
///
/// Each sub-module represents a bounded context with its own aggregate root,
/// value objects, entities, repository trait, and domain services.
///
/// ## Bounded Contexts
///
/// - [`account`]    — Account management, credentials, and holdings storage
/// - [`allocation`] — Portfolio allocation computation and snapshot management
/// - [`asset`]      — Asset definitions, pricing, and contract addresses
/// - [`chain`]      — Blockchain configurations, EVM chains, tokens
/// - [`portfolio`]  — Portfolio management, target allocations, and snapshots

pub mod account;
pub mod allocation;
pub mod asset;
pub mod chain;
pub mod portfolio;

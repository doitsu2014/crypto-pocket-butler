/// Account Domain
///
/// Bounded Context: Account management, credentials, and holdings storage.
/// Aggregate Root: Account
///
/// This module defines the core Account domain with:
/// - `Account` aggregate root
/// - `AccountType` value object (Exchange vs Wallet)
/// - `AccountCredentials` value object (encrypted API keys)
/// - `AccountHolding` entity (per-asset quantity data)
/// - `AccountHoldings` value object (collection of holdings)
/// - `AccountRepository` trait (persistence interface)

pub mod aggregate;
pub mod value_objects;
pub mod entities;
pub mod repository;
pub mod service;

pub use aggregate::Account;
pub use value_objects::{AccountType, AccountCredentials};
pub use entities::{AccountHolding, AccountHoldings};
pub use repository::AccountRepository;

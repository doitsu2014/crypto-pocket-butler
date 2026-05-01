/// Chain & Token Domain
///
/// Bounded Context: Blockchain configurations, EVM chains, EVM tokens, Solana tokens.
/// Aggregate Root: EvmChain
///
/// This module defines:
/// - `EvmChain` aggregate root  
/// - `EvmToken` entity
/// - `SolanaToken` entity
/// - `ChainRepository` trait

pub mod aggregate;
pub mod entities;
pub mod repository;

pub use aggregate::EvmChain;
pub use entities::{EvmToken, SolanaToken};
pub use repository::ChainRepository;

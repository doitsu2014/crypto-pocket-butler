//! Domain errors. Every refusal names the rule it enforces.

/// A business-rule violation, expressed without any transport or storage detail.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A symbol was empty after trimming.
    #[error("symbol must not be empty")]
    EmptySymbol,

    /// A symbol contained whitespace or control characters, or was too long.
    #[error("symbol is not a valid ticker: {0}")]
    InvalidSymbol(String),

    /// A display name was empty.
    #[error("name must not be empty")]
    EmptyName,

    /// Canonical precision must be between 0 and 36.
    #[error("decimals must be between 0 and 36, got {0}")]
    DecimalsOutOfRange(u8),

    /// BR1.2: a contract's decimals must match its canonical asset's decimals.
    #[error("contract decimals {contract} do not match asset decimals {asset}")]
    DecimalsMismatch { asset: u8, contract: u8 },

    /// A contract address was blank or contained forbidden characters.
    #[error("invalid contract address: {0}")]
    InvalidContractAddress(String),

    /// A chain family was not one of the launch families.
    #[error("invalid chain family: {0}")]
    InvalidChainFamily(String),

    /// A price source has no licence reference and may not be enabled for production.
    #[error("price source '{0}' has no licence reference and cannot be enabled")]
    UnlicensedSource(String),

    /// A priority must be at least 1.
    #[error("priority must be at least 1, got {0}")]
    InvalidPriority(u32),

    /// BR2.1: at most one enabled source may hold a given priority.
    #[error("priority {0} is already held by another enabled price source")]
    DuplicatePriority(u32),

    /// A state-machine transition was not allowed.
    #[error("invalid observation transition from {from} to {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },

    /// An ignore action must record why.
    #[error("ignore reason must not be empty")]
    EmptyIgnoreReason,

    /// BR2.2: a usable price is strictly positive; zero and negatives are not prices.
    #[error("invalid price: {0}")]
    InvalidPrice(String),

    /// An identifier was blank, too long, or contained whitespace/control characters.
    #[error("invalid identifier: {0}")]
    InvalidId(String),
}

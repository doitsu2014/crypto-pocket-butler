//! Chains bound what the registry observes.

use crate::error::DomainError;
use crate::ids::ChainId;

/// The chain families in the launch coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainFamily {
    /// EVM-compatible chains (Ethereum and peers).
    Evm,
    /// BNB Smart Chain.
    Bnb,
    /// Solana.
    Solana,
}

impl ChainFamily {
    /// Parses the stored family value.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidChainFamily`] for anything outside launch coverage.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "evm" => Ok(Self::Evm),
            "bnb" => Ok(Self::Bnb),
            "solana" => Ok(Self::Solana),
            other => Err(DomainError::InvalidChainFamily(other.to_string())),
        }
    }

    /// The stored family value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Evm => "evm",
            Self::Bnb => "bnb",
            Self::Solana => "solana",
        }
    }
}

/// A network whose addresses and contracts are observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    id: ChainId,
    name: String,
    family: ChainFamily,
    launch_coverage: bool,
}

impl Chain {
    /// Creates a chain.
    ///
    /// # Errors
    /// Returns a [`DomainError`] when the identifier or name is blank.
    pub fn new(
        id: impl Into<String>,
        name: &str,
        family: ChainFamily,
        launch_coverage: bool,
    ) -> Result<Self, DomainError> {
        let id = ChainId::new(id)?;
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id,
            name: name.to_string(),
            family,
            launch_coverage,
        })
    }

    /// The stable opaque identifier.
    #[must_use]
    pub fn id(&self) -> &ChainId {
        &self.id
    }

    /// The display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The chain family.
    #[must_use]
    pub fn family(&self) -> ChainFamily {
        self.family
    }

    /// Whether observations on this chain count toward portfolios.
    #[must_use]
    pub fn launch_coverage(&self) -> bool {
        self.launch_coverage
    }
}

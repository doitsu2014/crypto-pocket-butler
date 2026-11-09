//! The canonical asset: one holdable thing, independent of how many chains or
//! contracts represent it.

use crate::error::DomainError;
use crate::ids::AssetId;
use crate::symbol::normalize_symbol;

/// Canonical precision bounds. Decimals above 36 do not occur on the launch
/// networks and are treated as a registry error.
pub const MAX_DECIMALS: u8 = 36;

/// A canonical asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    id: AssetId,
    symbol: String,
    name: String,
    decimals: u8,
    canonical: bool,
}

impl Asset {
    /// Creates a canonical asset.
    ///
    /// # Errors
    /// Returns a [`DomainError`] when the identifier, symbol, name, or decimals
    /// violate their invariants.
    pub fn new(
        id: impl Into<String>,
        symbol: &str,
        name: &str,
        decimals: u8,
        canonical: bool,
    ) -> Result<Self, DomainError> {
        let id = AssetId::new(id)?;
        let symbol = normalize_symbol(symbol)?;
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::EmptyName);
        }
        if decimals > MAX_DECIMALS {
            return Err(DomainError::DecimalsOutOfRange(decimals));
        }
        Ok(Self {
            id,
            symbol,
            name: name.to_string(),
            decimals,
            canonical,
        })
    }

    /// The stable opaque identifier.
    #[must_use]
    pub fn id(&self) -> &AssetId {
        &self.id
    }

    /// The normalized uppercase ticker.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// The display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The canonical precision used for every quantity of this asset.
    #[must_use]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    /// Whether this is a real registry entry rather than a placeholder.
    #[must_use]
    pub fn canonical(&self) -> bool {
        self.canonical
    }
}

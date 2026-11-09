//! Token contracts: how one chain expresses a canonical asset (BR1.1, BR1.2).

use std::fmt;

use crate::asset::{Asset, MAX_DECIMALS};
use crate::chain::ChainFamily;
use crate::error::DomainError;
use crate::ids::{AssetId, ChainId, TokenContractId};

const MAX_ADDRESS_LEN: usize = 128;

/// An opaque contract address. Case and checksum are preserved; chain-family
/// normalization is computed only for lookup and uniqueness.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContractAddress(String);

impl ContractAddress {
    /// Parses a contract address.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidContractAddress`] when the value is blank,
    /// too long, or contains whitespace or control characters.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(DomainError::InvalidContractAddress(
                "address is empty".to_string(),
            ));
        }
        if value.len() > MAX_ADDRESS_LEN {
            return Err(DomainError::InvalidContractAddress(format!(
                "address is longer than {MAX_ADDRESS_LEN} characters"
            )));
        }
        if value.chars().any(char::is_whitespace) || value.chars().any(char::is_control) {
            return Err(DomainError::InvalidContractAddress(
                "address must not contain whitespace or control characters".to_string(),
            ));
        }
        Ok(Self(value.to_string()))
    }

    /// The address exactly as observed (case and checksum preserved).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The lookup key for a chain family. EVM and BNB addresses are compared
    /// case-insensitively; Solana base58 addresses are compared exactly.
    #[must_use]
    pub fn key_for(&self, family: ChainFamily) -> String {
        match family {
            ChainFamily::Evm | ChainFamily::Bnb => self.0.to_ascii_lowercase(),
            ChainFamily::Solana => self.0.clone(),
        }
    }
}

impl fmt::Display for ContractAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A contract address on one chain that represents a canonical asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenContract {
    id: TokenContractId,
    chain_id: ChainId,
    contract_address: ContractAddress,
    asset_id: AssetId,
    decimals: u8,
}

impl TokenContract {
    /// Creates a token contract.
    ///
    /// # Errors
    /// Returns a [`DomainError`] when the identifier is invalid or decimals are
    /// outside the allowed range.
    pub fn new(
        id: impl Into<String>,
        chain_id: &ChainId,
        contract_address: ContractAddress,
        asset_id: &AssetId,
        decimals: u8,
    ) -> Result<Self, DomainError> {
        let id = TokenContractId::new(id)?;
        if decimals > MAX_DECIMALS {
            return Err(DomainError::DecimalsOutOfRange(decimals));
        }
        Ok(Self {
            id,
            chain_id: chain_id.clone(),
            contract_address,
            asset_id: asset_id.clone(),
            decimals,
        })
    }

    /// The stable opaque identifier.
    #[must_use]
    pub fn id(&self) -> &TokenContractId {
        &self.id
    }

    /// The chain this contract lives on.
    #[must_use]
    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    /// The contract address as observed.
    #[must_use]
    pub fn contract_address(&self) -> &ContractAddress {
        &self.contract_address
    }

    /// The canonical asset this contract resolves to.
    #[must_use]
    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    /// The contract precision, which must equal the asset's precision.
    #[must_use]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

/// BR1.2: a contract's decimals must match its canonical asset's decimals.
///
/// # Errors
/// Returns [`DomainError::DecimalsMismatch`] naming both values.
pub fn ensure_decimals_match(asset: &Asset, contract_decimals: u8) -> Result<(), DomainError> {
    if asset.decimals() == contract_decimals {
        Ok(())
    } else {
        Err(DomainError::DecimalsMismatch {
            asset: asset.decimals(),
            contract: contract_decimals,
        })
    }
}

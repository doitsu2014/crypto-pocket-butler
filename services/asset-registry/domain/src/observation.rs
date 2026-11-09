//! The quarantine for observations the registry cannot resolve (BR1.1, BR1.3).

use chrono::{DateTime, Utc};

use crate::contract::ContractAddress;
use crate::error::DomainError;
use crate::ids::{AssetId, ChainId, ObservationId};
use crate::symbol::normalize_symbol;

/// The state of an unmapped observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationStatus {
    /// Excluded from every portfolio total until an operator acts.
    Quarantined,
    /// Mapped to a canonical asset; later observations reuse the mapping.
    Resolved {
        /// The canonical asset the observation was mapped to.
        asset_id: AssetId,
    },
    /// Deliberately excluded; later observations of the same token stay excluded.
    Ignored {
        /// Why the operator ignored it.
        reason: String,
    },
}

impl ObservationStatus {
    /// A stable label for transitions and storage.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Quarantined => "quarantined",
            Self::Resolved { .. } => "resolved",
            Self::Ignored { .. } => "ignored",
        }
    }
}

/// A token observed in an account or chain that no canonical asset resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmappedObservation {
    id: ObservationId,
    observed_symbol: String,
    chain_id: ChainId,
    contract_address: Option<ContractAddress>,
    first_seen_at: DateTime<Utc>,
    status: ObservationStatus,
}

impl UnmappedObservation {
    /// Creates a quarantined observation.
    ///
    /// # Errors
    /// Returns a [`DomainError`] for an invalid identifier or symbol.
    pub fn quarantine(
        id: impl Into<String>,
        observed_symbol: &str,
        chain_id: &ChainId,
        contract_address: Option<ContractAddress>,
        first_seen_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let id = ObservationId::new(id)?;
        let observed_symbol = normalize_symbol(observed_symbol)?;
        Ok(Self {
            id,
            observed_symbol,
            chain_id: chain_id.clone(),
            contract_address,
            first_seen_at,
            status: ObservationStatus::Quarantined,
        })
    }

    /// Maps a quarantined observation to an asset.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the observation is ignored
    /// or already mapped to a different asset.
    pub fn resolve(&mut self, asset_id: AssetId) -> Result<(), DomainError> {
        match &self.status {
            ObservationStatus::Quarantined => {
                self.status = ObservationStatus::Resolved { asset_id };
                Ok(())
            }
            ObservationStatus::Resolved { asset_id: existing } if *existing == asset_id => Ok(()),
            current => Err(DomainError::InvalidTransition {
                from: current.label(),
                to: "resolved",
            }),
        }
    }

    /// Ignores a quarantined observation and records why.
    ///
    /// # Errors
    /// Returns [`DomainError::EmptyIgnoreReason`] for a blank reason and
    /// [`DomainError::InvalidTransition`] when the observation is resolved.
    pub fn ignore(&mut self, reason: &str) -> Result<(), DomainError> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(DomainError::EmptyIgnoreReason);
        }
        match &self.status {
            ObservationStatus::Quarantined => {
                self.status = ObservationStatus::Ignored {
                    reason: reason.to_string(),
                };
                Ok(())
            }
            ObservationStatus::Ignored { reason: existing } if existing == reason => Ok(()),
            current => Err(DomainError::InvalidTransition {
                from: current.label(),
                to: "ignored",
            }),
        }
    }

    /// The stable opaque identifier.
    #[must_use]
    pub fn id(&self) -> &ObservationId {
        &self.id
    }

    /// The normalized observed symbol.
    #[must_use]
    pub fn observed_symbol(&self) -> &str {
        &self.observed_symbol
    }

    /// The chain the observation was seen on.
    #[must_use]
    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    /// The observed contract address, when the observation was a token.
    #[must_use]
    pub fn contract_address(&self) -> Option<&ContractAddress> {
        self.contract_address.as_ref()
    }

    /// When the observation was first seen.
    #[must_use]
    pub fn first_seen_at(&self) -> DateTime<Utc> {
        self.first_seen_at
    }

    /// The current state.
    #[must_use]
    pub fn status(&self) -> &ObservationStatus {
        &self.status
    }
}

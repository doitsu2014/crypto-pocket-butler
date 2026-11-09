//! The freshness tier of one canonical asset (WF4, BR2.1).
//!
//! The canonical asset list and each asset's rank are Asset Registry data. A
//! tier row copies the rank the last refresh read; an asset with no row is
//! treated as `tail` by the read path.

use chrono::{DateTime, Utc};

use crate::constants::{MAX_ASSET_ID_CHARS, TAIL_BOUND_SECONDS, TOP100_BOUND_SECONDS};
use crate::error::DomainError;

/// The freshness tier of an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceTier {
    /// Rank 1–100, refreshed every five minutes with a 300-second bound.
    Top100,
    /// Everything else, refreshed every ten minutes with a 900-second bound.
    Tail,
}

impl PriceTier {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Top100 => "top100",
            Self::Tail => "tail",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown tier.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "top100" => Ok(Self::Top100),
            "tail" => Ok(Self::Tail),
            _ => Err(DomainError::Invalid {
                field: "tier",
                reason: "must be top100 or tail".to_string(),
            }),
        }
    }

    /// The freshness bound this tier imposes.
    #[must_use]
    pub fn bound_seconds(self) -> i64 {
        match self {
            Self::Top100 => TOP100_BOUND_SECONDS,
            Self::Tail => TAIL_BOUND_SECONDS,
        }
    }
}

/// The bound a rank imposes: 300 seconds for 1–100, 900 otherwise.
#[must_use]
pub fn bound_for_rank(rank: u32) -> i64 {
    if rank <= 100 {
        TOP100_BOUND_SECONDS
    } else {
        TAIL_BOUND_SECONDS
    }
}

/// Validates and trims a canonical asset id.
///
/// # Errors
/// Returns [`DomainError::Required`] for a blank id and
/// [`DomainError::TooLarge`] above [`MAX_ASSET_ID_CHARS`].
pub fn validate_asset_id(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "assetId" });
    }
    if value.chars().count() > MAX_ASSET_ID_CHARS {
        return Err(DomainError::TooLarge {
            field: "assetId",
            limit: "200 characters",
        });
    }
    Ok(value.to_string())
}

/// One canonical asset's freshness tier, refreshed from C4 at the start of a
/// run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPriceTier {
    asset_id: String,
    rank: u32,
    tier: PriceTier,
    bound_seconds: i64,
    updated_at: DateTime<Utc>,
}

impl AssetPriceTier {
    /// Derives the tier of one asset from its canonical rank.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank asset id and
    /// [`DomainError::Invalid`] for a rank below one.
    pub fn for_rank(
        asset_id: &str,
        rank: u32,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let asset_id = validate_asset_id(asset_id)?;
        if rank == 0 {
            return Err(DomainError::Invalid {
                field: "rank",
                reason: "must be at least 1".to_string(),
            });
        }
        let tier = if rank <= 100 {
            PriceTier::Top100
        } else {
            PriceTier::Tail
        };
        Ok(Self {
            asset_id,
            rank,
            tier,
            bound_seconds: tier.bound_seconds(),
            updated_at,
        })
    }

    /// Rehydrates a stored tier row.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the stored tier or bound disagrees
    /// with the rank.
    pub fn from_stored(
        asset_id: String,
        rank: u32,
        tier: PriceTier,
        bound_seconds: i64,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let expected_tier = if rank <= 100 {
            PriceTier::Top100
        } else {
            PriceTier::Tail
        };
        if tier != expected_tier || bound_seconds != expected_tier.bound_seconds() {
            return Err(DomainError::Invalid {
                field: "tier",
                reason: "does not match the stored rank".to_string(),
            });
        }
        Self::for_rank(&asset_id, rank, updated_at)
    }

    /// The canonical asset id.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The canonical rank copied from C4.
    #[must_use]
    pub fn rank(&self) -> u32 {
        self.rank
    }

    /// The tier.
    #[must_use]
    pub fn tier(&self) -> PriceTier {
        self.tier
    }

    /// The freshness bound.
    #[must_use]
    pub fn bound_seconds(&self) -> i64 {
        self.bound_seconds
    }

    /// When the row was refreshed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

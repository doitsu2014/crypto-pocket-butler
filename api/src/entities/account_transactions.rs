use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// **Accounts domain** — append-only audit record of every balance observation
/// for a single asset within an exchange/wallet account.
///
/// Each row is written by the account-sync job when a balance is fetched from
/// an exchange API or on-chain source.  The *current* balance for a given
/// `(account_id, asset_symbol)` pair is the `quantity_after` of the row with
/// the highest `created_at` for that pair.
///
/// # Audit fields
/// * `created_at`       – when this observation was recorded
/// * `updated_at`       – last modification timestamp (normally equals `created_at`)
/// * `transaction_type` – semantic label: "sync" | "deposit" | "withdrawal" | "manual_adjustment"
/// * `source`           – data origin: "okx" | "ethereum" | "solana" | "manual" | …
/// * `metadata`         – optional JSON for extra context (job ID, user ID, …)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "account_transactions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_id: Uuid,
    /// Asset symbol as reported by the exchange/chain (e.g. "BTC", "ETH-ethereum").
    pub asset_symbol: String,
    /// Balance *before* this observation (normalized decimal string).
    pub quantity_before: String,
    /// Balance *after* this observation (normalized decimal string).
    pub quantity_after: String,
    /// Signed delta: `quantity_after − quantity_before` (normalized decimal string).
    pub quantity_change: String,
    /// Semantic label for what caused the balance change.
    pub transaction_type: String,
    /// The data source that produced this observation.
    pub source: String,
    /// Optional freeform JSON for additional audit context.
    pub metadata: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::accounts::Entity",
        from = "Column::AccountId",
        to = "super::accounts::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Accounts,
}

impl Related<super::accounts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Accounts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

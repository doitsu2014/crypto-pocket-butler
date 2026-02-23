use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Append-only audit record for every balance change in a [`super::holdings::Model`].
///
/// Replaying all transactions for a holding (ordered by `created_at` ascending)
/// will always reproduce the current `holdings.quantity`.
///
/// # Audit fields
/// * `created_at`       – when this transaction was first recorded
/// * `updated_at`       – last time the record was modified (normally equals `created_at`)
/// * `transaction_type` – semantic label for the change (e.g. "sync", "deposit", "withdrawal")
/// * `source`           – data origin (e.g. "okx", "ethereum", "manual")
/// * `metadata`         – optional JSON with additional context (job ID, user ID, etc.)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "holding_transactions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub holding_id: Uuid,
    /// Balance immediately before this event (normalized decimal string).
    pub quantity_before: String,
    /// Balance immediately after this event (normalized decimal string).
    pub quantity_after: String,
    /// Signed delta: `quantity_after − quantity_before` (normalized decimal string).
    pub quantity_change: String,
    /// Semantic label: "sync", "deposit", "withdrawal", "manual_adjustment", etc.
    pub transaction_type: String,
    /// Data source: "okx", "ethereum", "solana", "manual", etc.
    pub source: String,
    /// Optional freeform JSON for extra audit context.
    pub metadata: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::holdings::Entity",
        from = "Column::HoldingId",
        to = "super::holdings::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Holdings,
}

impl Related<super::holdings::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Holdings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

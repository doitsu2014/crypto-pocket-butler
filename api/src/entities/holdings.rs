use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Current holding state for a single asset within an account.
///
/// The `quantity` field is always the most-recent reconstructed balance –
/// i.e. the running sum of all associated [`super::holding_transactions::Model`] entries.
///
/// # Invariant
/// `quantity` must always equal the `quantity_after` of the latest transaction
/// for this holding.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "holdings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_id: Uuid,
    /// Asset symbol as reported by the exchange or chain (e.g. "BTC", "ETH-ethereum").
    pub asset_symbol: String,
    /// Current quantity as a normalized (human-readable) decimal string.
    pub quantity: String,
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
    #[sea_orm(has_many = "super::holding_transactions::Entity")]
    HoldingTransactions,
}

impl Related<super::accounts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Accounts.def()
    }
}

impl Related<super::holding_transactions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HoldingTransactions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

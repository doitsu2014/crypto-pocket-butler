use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// **Accounts domain** — current holding for a single asset within an
/// exchange or wallet account.
///
/// One row exists per `(account_id, asset_symbol)`.  The `quantity` field is
/// **overwritten** on every sync run, so this table always reflects the
/// most-recent balance fetched from the exchange API or on-chain source.
///
/// For a full audit trail of balance changes over time, consumers should
/// compare the JSON column `accounts.holdings` across snapshots, or extend
/// this model with a separate history table in the future.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "account_holdings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_id: Uuid,
    /// Asset symbol as reported by the exchange/chain (e.g. "BTC", "ETH-ethereum").
    pub asset_symbol: String,
    /// Current normalized balance (overwritten on every sync).
    pub quantity: String,
    /// Data source that produced the latest balance value.
    pub source: String,
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

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// **Portfolios domain** — current aggregated holding for a single asset within
/// a portfolio.
///
/// One row exists per `(portfolio_id, asset_symbol)`.  The `quantity` field is
/// updated whenever the portfolio allocation is (re-)constructed, accumulating
/// the latest balance across all accounts linked to the portfolio.
///
/// Historical valuations are stored in [`super::holding_valuations::Entity`].
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "portfolio_holdings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub portfolio_id: Uuid,
    /// Canonical asset symbol (e.g. "BTC", "ETH").
    pub asset_symbol: String,
    /// Current aggregated quantity across all linked accounts (normalized decimal string).
    pub quantity: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::portfolios::Entity",
        from = "Column::PortfolioId",
        to = "super::portfolios::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Portfolios,
    #[sea_orm(has_many = "super::holding_valuations::Entity")]
    HoldingValuations,
}

impl Related<super::portfolios::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Portfolios.def()
    }
}

impl Related<super::holding_valuations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HoldingValuations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

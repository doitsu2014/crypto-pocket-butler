use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// **Portfolios domain** — daily time-series valuation for a portfolio holding.
///
/// One row is written per `(portfolio_holding_id, date)` each time the
/// portfolio allocation is constructed.  Replaying all rows for a holding in
/// ascending date order gives a complete history of its USD value.
///
/// # Performance analysis
/// Query `holding_valuations` grouped by `date` to plot daily portfolio value
/// or compare performance across time periods.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "holding_valuations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub portfolio_holding_id: Uuid,
    /// Calendar date of this valuation snapshot (UTC).
    pub date: Date,
    /// Asset price used for this valuation (from `asset_prices`).
    pub price_usd: Decimal,
    /// Quantity of the holding on this date.
    pub quantity: Decimal,
    /// Computed value: `price_usd × quantity`.
    pub value_usd: Decimal,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::portfolio_holdings::Entity",
        from = "Column::PortfolioHoldingId",
        to = "super::portfolio_holdings::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    PortfolioHoldings,
}

impl Related<super::portfolio_holdings::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PortfolioHoldings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

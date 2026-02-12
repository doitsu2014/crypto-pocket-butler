use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "recommendations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub portfolio_id: Uuid,
    pub status: String, // "pending", "approved", "rejected", "executed"
    pub recommendation_type: String, // "rebalance", "take_profit", "stop_loss"
    pub rationale: String,
    pub proposed_orders: Json, // Array of order objects: [{action, asset, quantity, estimated_price, estimated_value_usd}]
    pub expected_impact: Option<Decimal>,
    pub metadata: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub executed_at: Option<DateTimeWithTimeZone>,
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
}

impl Related<super::portfolios::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Portfolios.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

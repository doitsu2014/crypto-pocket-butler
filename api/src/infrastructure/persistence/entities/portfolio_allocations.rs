use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "portfolio_allocations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub portfolio_id: Uuid,
    pub as_of: DateTimeWithTimeZone,
    pub total_value_usd: Decimal,
    pub holdings: Json, // JSON array of asset holdings with values and weights
    pub created_at: DateTimeWithTimeZone,
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

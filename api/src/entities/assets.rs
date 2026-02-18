use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "assets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub symbol: String,
    pub name: String,
    pub asset_type: String, // "cryptocurrency", "token", "stablecoin"
    pub coingecko_id: Option<String>,
    pub coinmarketcap_id: Option<String>,
    pub logo_url: Option<String>,
    pub description: Option<String>,
    pub decimals: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::asset_contracts::Entity")]
    AssetContracts,
    #[sea_orm(has_many = "super::asset_prices::Entity")]
    AssetPrices,
}

impl Related<super::asset_contracts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AssetContracts.def()
    }
}

impl Related<super::asset_prices::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AssetPrices.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

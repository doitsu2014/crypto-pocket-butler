use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "asset_prices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub asset_id: Uuid,
    pub timestamp: DateTimeWithTimeZone,
    pub price_usd: Decimal,
    pub volume_24h_usd: Option<Decimal>,
    pub market_cap_usd: Option<Decimal>,
    pub change_percent_24h: Option<Decimal>,
    pub source: String, // e.g., "coinpaprika", "coinmarketcap"
    pub created_at: DateTimeWithTimeZone,
    // New fields from CoinPaprika consolidation
    pub rank: Option<i32>, // Market cap rank
    pub circulating_supply: Option<Decimal>,
    pub total_supply: Option<Decimal>,
    pub max_supply: Option<Decimal>,
    pub beta_value: Option<Decimal>,
    pub percent_change_1h: Option<Decimal>,
    pub percent_change_7d: Option<Decimal>,
    pub percent_change_30d: Option<Decimal>,
    pub ath_price: Option<Decimal>, // All-time high price
    pub ath_date: Option<DateTimeWithTimeZone>,
    pub percent_from_price_ath: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::assets::Entity",
        from = "Column::AssetId",
        to = "super::assets::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Assets,
}

impl Related<super::assets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Assets.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

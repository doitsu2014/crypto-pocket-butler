use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "evm_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// EVM chain identifier, e.g. "ethereum", "arbitrum", "bsc"
    pub chain: String,
    /// Token symbol, e.g. "USDC", "WBTC"
    pub symbol: String,
    /// ERC-20 contract address (checksummed hex)
    pub contract_address: String,
    /// Whether the EVM connector should include this token during account sync
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

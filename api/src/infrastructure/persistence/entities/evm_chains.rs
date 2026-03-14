use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "evm_chains")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Unique chain identifier, e.g. "ethereum", "bsc"
    pub chain_id: String,
    /// Human-readable chain name, e.g. "Ethereum"
    pub name: String,
    /// RPC URL used to connect to this chain
    pub rpc_url: String,
    /// Native token symbol, e.g. "ETH", "BNB", "HYPE"
    pub native_symbol: String,
    /// Whether this chain is active for account sync
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

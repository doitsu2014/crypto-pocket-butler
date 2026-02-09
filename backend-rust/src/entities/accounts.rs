use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub account_type: String, // "exchange", "wallet", "defi"
    pub exchange_name: Option<String>,
    #[serde(skip_serializing)] // Don't expose in API responses
    pub api_key_encrypted: Option<String>,
    #[serde(skip_serializing)] // Don't expose in API responses
    pub api_secret_encrypted: Option<String>,
    #[serde(skip_serializing)] // Don't expose in API responses
    pub passphrase_encrypted: Option<String>,
    pub wallet_address: Option<String>,
    pub is_active: bool,
    pub last_synced_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users,
    #[sea_orm(has_many = "super::portfolio_accounts::Entity")]
    PortfolioAccounts,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl Related<super::portfolio_accounts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PortfolioAccounts.def()
    }
}

// Many-to-many relation with portfolios through portfolio_accounts
impl Related<super::portfolios::Entity> for Entity {
    fn to() -> RelationDef {
        super::portfolio_accounts::Relation::Portfolios.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::portfolio_accounts::Relation::Accounts.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

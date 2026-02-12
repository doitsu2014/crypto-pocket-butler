use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "portfolios")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub target_allocation: Option<serde_json::Value>,
    pub guardrails: Option<serde_json::Value>,
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
    #[sea_orm(has_many = "super::snapshots::Entity")]
    Snapshots,
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

impl Related<super::snapshots::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Snapshots.def()
    }
}

// Many-to-many relation with accounts through portfolio_accounts
impl Related<super::accounts::Entity> for Entity {
    fn to() -> RelationDef {
        super::portfolio_accounts::Relation::Accounts.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::portfolio_accounts::Relation::Portfolios.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

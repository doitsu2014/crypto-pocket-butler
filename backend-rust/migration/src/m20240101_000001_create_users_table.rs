use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(uuid(Users::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(string(Users::KeycloakUserId).unique_key().not_null())
                    .col(string_null(Users::Email))
                    .col(string_null(Users::PreferredUsername))
                    .col(timestamp_with_time_zone(Users::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Users::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .to_owned(),
            )
            .await?;

        // Create index on keycloak_user_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_users_keycloak_user_id")
                    .table(Users::Table)
                    .col(Users::KeycloakUserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    KeycloakUserId,
    Email,
    PreferredUsername,
    CreatedAt,
    UpdatedAt,
}

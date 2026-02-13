use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create users table
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
            .await?;

        // Create accounts table with all columns including holdings and enabled_chains
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(uuid(Accounts::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(Accounts::UserId).not_null())
                    .col(string(Accounts::Name).not_null())
                    .col(string(Accounts::AccountType).not_null()) // e.g., "exchange", "wallet", "defi"
                    .col(string_null(Accounts::ExchangeName)) // e.g., "okx", "binance", "coinbase"
                    .col(string_null(Accounts::ApiKeyEncrypted))
                    .col(string_null(Accounts::ApiSecretEncrypted))
                    .col(string_null(Accounts::PassphraseEncrypted))
                    .col(string_null(Accounts::WalletAddress))
                    .col(boolean(Accounts::IsActive).default(true).not_null())
                    .col(timestamp_with_time_zone_null(Accounts::LastSyncedAt)) // Nullable from the start
                    .col(json_null(Accounts::Holdings)) // Added from m000006
                    .col(json_null(Accounts::EnabledChains)) // Added from m000014
                    .col(timestamp_with_time_zone(Accounts::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(Accounts::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_user_id")
                            .from(Accounts::Table, Accounts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_accounts_user_id")
                    .table(Accounts::Table)
                    .col(Accounts::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on account_type for filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_accounts_account_type")
                    .table(Accounts::Table)
                    .col(Accounts::AccountType)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Accounts::Table).to_owned())
            .await?;
        
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

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
    UserId,
    Name,
    AccountType,
    ExchangeName,
    ApiKeyEncrypted,
    ApiSecretEncrypted,
    PassphraseEncrypted,
    WalletAddress,
    IsActive,
    LastSyncedAt,
    Holdings,
    EnabledChains,
    CreatedAt,
    UpdatedAt,
}

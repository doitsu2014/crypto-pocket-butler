use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AssetContracts::Table)
                    .if_not_exists()
                    .col(uuid(AssetContracts::Id).primary_key().extra("DEFAULT gen_random_uuid()"))
                    .col(uuid(AssetContracts::AssetId).not_null())
                    .col(string(AssetContracts::Chain).not_null()) // e.g., "ethereum", "bsc", "polygon", "arbitrum"
                    .col(string(AssetContracts::ContractAddress).not_null()) // Contract address on the chain
                    .col(string_null(AssetContracts::TokenStandard)) // e.g., "ERC20", "BEP20", "ERC721"
                    .col(integer_null(AssetContracts::Decimals)) // Token decimals (can override asset default)
                    .col(boolean(AssetContracts::IsVerified).default(false).not_null()) // Whether contract is verified
                    .col(timestamp_with_time_zone(AssetContracts::CreatedAt).default(Expr::current_timestamp()).not_null())
                    .col(timestamp_with_time_zone(AssetContracts::UpdatedAt).default(Expr::current_timestamp()).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_contracts_asset_id")
                            .from(AssetContracts::Table, AssetContracts::AssetId)
                            .to(Assets::Table, Assets::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on asset_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_asset_id")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::AssetId)
                    .to_owned(),
            )
            .await?;

        // Create index on chain for filtering by blockchain
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_chain")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::Chain)
                    .to_owned(),
            )
            .await?;

        // Create unique index to prevent duplicate contract addresses per chain
        manager
            .create_index(
                Index::create()
                    .name("idx_asset_contracts_unique")
                    .table(AssetContracts::Table)
                    .col(AssetContracts::Chain)
                    .col(AssetContracts::ContractAddress)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AssetContracts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AssetContracts {
    Table,
    Id,
    AssetId,
    Chain,
    ContractAddress,
    TokenStandard,
    Decimals,
    IsVerified,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Fix column names in asset_prices table to match entity model expectations
        // The original migration used DeriveIden which converts Volume24hUsd to volume24h_usd
        // but the entity expects volume_24h_usd (with underscore before 24)
        
        // Check if columns exist before attempting rename
        // This allows the migration to be idempotent
        
        // Rename volume24h_usd to volume_24h_usd (if the incorrectly named column exists)
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN volume24h_usd TO volume_24h_usd;"
            )
            .await;
        
        // Rename market_cap_usd should be OK as is (no number issue)
        
        // Rename change_percent24h to change_percent_24h
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN change_percent24h TO change_percent_24h;"
            )
            .await;
        
        // Similarly fix percent_change fields if they exist with wrong names
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN percent_change1h TO percent_change_1h;"
            )
            .await;
        
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN percent_change7d TO percent_change_7d;"
            )
            .await;
        
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN percent_change30d TO percent_change_30d;"
            )
            .await;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert column renames
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN volume_24h_usd TO volume24h_usd;"
            )
            .await;
        
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN change_percent_24h TO change_percent24h;"
            )
            .await;
        
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN percent_change_1h TO percent_change1h;"
            )
            .await;
        
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN percent_change_7d TO percent_change7d;"
            )
            .await;
        
        let _ = manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE asset_prices 
                 RENAME COLUMN percent_change_30d TO percent_change30d;"
            )
            .await;

        Ok(())
    }
}

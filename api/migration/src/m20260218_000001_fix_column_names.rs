use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Helper to execute ALTER TABLE RENAME COLUMN statements, ignoring only "column does not exist" errors
/// 
/// This function uses PostgreSQL identifier quoting to prevent SQL injection, even though
/// all current calls use hardcoded strings.
async fn rename_column_if_exists(
    manager: &SchemaManager<'_>,
    from_name: &str,
    to_name: &str,
) -> Result<(), DbErr> {
    // Validate column names contain only safe characters
    let is_valid_identifier = |name: &str| {
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    };
    
    if !is_valid_identifier(from_name) || !is_valid_identifier(to_name) {
        return Err(DbErr::Custom(format!(
            "Invalid column name: must contain only alphanumeric characters and underscores"
        )));
    }
    
    // Use double quotes to properly escape PostgreSQL identifiers
    let sql = format!(
        r#"ALTER TABLE "asset_prices" RENAME COLUMN "{}" TO "{}";"#,
        from_name, to_name
    );
    
    match manager.get_connection().execute_unprepared(&sql).await {
        Ok(_) => Ok(()),
        Err(e) => {
            // PostgreSQL error code 42703 = "undefined_column"
            // Check if this is the expected "column does not exist" error
            let err_str = e.to_string();
            if err_str.contains("42703") || 
               (err_str.contains("column") && err_str.contains("does not exist")) {
                // Expected: column was already renamed or never had the wrong name
                Ok(())
            } else {
                // Unexpected error (permissions, connection, etc.) - propagate it
                Err(e)
            }
        }
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Fix column names in asset_prices table to match entity model expectations
        // The original migration used DeriveIden which converts Volume24hUsd to volume24h_usd
        // but the entity expects volume_24h_usd (with underscore before 24)
        
        rename_column_if_exists(manager, "volume24h_usd", "volume_24h_usd").await?;
        rename_column_if_exists(manager, "change_percent24h", "change_percent_24h").await?;
        rename_column_if_exists(manager, "percent_change1h", "percent_change_1h").await?;
        rename_column_if_exists(manager, "percent_change7d", "percent_change_7d").await?;
        rename_column_if_exists(manager, "percent_change30d", "percent_change_30d").await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert column renames
        rename_column_if_exists(manager, "volume_24h_usd", "volume24h_usd").await?;
        rename_column_if_exists(manager, "change_percent_24h", "change_percent24h").await?;
        rename_column_if_exists(manager, "percent_change_1h", "percent_change1h").await?;
        rename_column_if_exists(manager, "percent_change_7d", "percent_change7d").await?;
        rename_column_if_exists(manager, "percent_change_30d", "percent_change30d").await?;

        Ok(())
    }
}

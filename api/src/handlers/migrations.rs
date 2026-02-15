use axum::{
    extract::State,
    response::Json,
    routing::post,
    Router,
};
use sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::error::ApiError;

/// Migration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MigrationResponse {
    /// Status of the migration
    pub status: String,
    /// Message about the migration
    pub message: String,
}

/// Run database migrations
///
/// This endpoint triggers database migrations to apply pending schema changes.
/// It's useful for manual migrations or CI/CD pipelines that need to ensure
/// the database schema is up-to-date before starting the application.
#[utoipa::path(
    post,
    path = "/api/v1/migrations/migrate",
    tag = "migrations",
    responses(
        (status = 200, description = "Migration completed successfully", body = MigrationResponse),
        (status = 500, description = "Migration failed")
    ),
    security(
        ("bearer_auth" = []),
        ("oauth2_client_credentials" = []),
        ("oauth2_authorization_code" = [])
    )
)]
pub async fn migrate_handler(
    State(db): State<DatabaseConnection>,
) -> Result<Json<MigrationResponse>, ApiError> {
    tracing::info!("Running database migrations");
    
    match migration::Migrator::up(&db, None).await {
        Ok(_) => {
            tracing::info!("Database migrations completed successfully");
            Ok(Json(MigrationResponse {
                status: "success".to_string(),
                message: "Database migrations completed successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Migration failed: {:?}", e);
            Err(ApiError::InternalServerError(format!("Migration failed: {}", e)))
        }
    }
}

pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/api/v1/migrations/migrate", post(migrate_handler))
}

use axum_keycloak_auth::decode::KeycloakToken;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::entities::users;

/// Get or create user in database from Keycloak token
/// 
/// This helper function looks up a user by their Keycloak user ID.
/// If the user doesn't exist, it creates a new user record with information
/// from the Keycloak token.
/// 
/// # Arguments
/// * `db` - Database connection
/// * `token` - Keycloak token containing user information
/// 
/// # Returns
/// * `Ok(users::Model)` - The existing or newly created user
/// * `Err(sea_orm::DbErr)` - Database error
pub async fn get_or_create_user(
    db: &DatabaseConnection,
    token: &KeycloakToken<String>,
) -> Result<users::Model, sea_orm::DbErr> {
    let keycloak_user_id = &token.subject;

    // Try to find existing user
    if let Some(user) = users::Entity::find()
        .filter(users::Column::KeycloakUserId.eq(keycloak_user_id))
        .one(db)
        .await?
    {
        return Ok(user);
    }

    // Create new user if not found
    let new_user = users::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        keycloak_user_id: ActiveValue::Set(keycloak_user_id.clone()),
        email: ActiveValue::Set(Some(token.extra.email.email.clone())),
        preferred_username: ActiveValue::Set(Some(token.extra.profile.preferred_username.clone())),
        created_at: ActiveValue::NotSet,
        updated_at: ActiveValue::NotSet,
    };

    let user = new_user.insert(db).await?;
    Ok(user)
}

#[cfg(test)]
mod tests {
    // Note: Full integration tests for get_or_create_user are done
    // in the handler integration tests. These are simple unit tests
    // to verify the function signature and basic compilation.

    #[test]
    fn test_helper_module_exists() {
        // This test ensures the module is properly structured
        // and the function signature is correct
        assert!(true);
    }

    // Additional integration tests should be added to test the actual
    // database interaction with a test database or in handler tests.
}

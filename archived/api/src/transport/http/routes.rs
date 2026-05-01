use axum::Router;
use sea_orm::DatabaseConnection;

use super::handlers;

/// Public routes that do not require authentication.
pub fn public_routes() -> Router<DatabaseConnection> {
    Router::new().merge(handlers::chains::create_router())
}

/// Protected routes that require a valid user token.
pub fn protected_routes() -> Router<DatabaseConnection> {
    Router::new()
        .merge(handlers::portfolios::create_router())
        .merge(handlers::accounts::create_router())
        .merge(handlers::snapshots::create_router())
        .merge(handlers::recommendations::create_router())
        .merge(handlers::migrations::create_router())
}

/// Admin-only routes that require the "administrator" role.
pub fn admin_routes() -> Router<DatabaseConnection> {
    Router::new()
        .merge(handlers::jobs::create_router())
        .merge(handlers::evm_tokens::create_router())
        .merge(handlers::evm_chains::create_router())
        .merge(handlers::solana_tokens::create_router())
}

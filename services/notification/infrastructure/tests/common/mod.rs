//! Disposable Postgres schema for the adapter suite.
//!
//! The suite needs `TEST_DATABASE_URL` and every test is `#[ignore]`d by default;
//! the unit suite runs without a database.

#![allow(dead_code)]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

/// Connects to a disposable Postgres database, creates a unique schema, and runs
/// the real migrations into it.
///
/// # Panics
/// Panics when `TEST_DATABASE_URL` is unset or the migrations fail.
pub async fn test_pool() -> (PgPool, String) {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must point at a disposable Postgres database for integration tests",
    );
    let schema = format!("test_notification_{}", Uuid::new_v4().simple());

    let admin = PgPool::connect(&url)
        .await
        .expect("connect to test database");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .expect("create disposable schema");
    admin.close().await;

    let schema_for_connection = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |connection, _meta| {
            let schema = schema_for_connection.clone();
            Box::pin(async move {
                sqlx::query(&format!("SET search_path TO {schema}"))
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("connect disposable pool");

    // Role creation is cluster-wide, so migrations are serialised across test
    // schemas with an advisory lock.
    let mut lock = pool.acquire().await.expect("acquire migration lock");
    sqlx::query("SELECT pg_advisory_lock(9812462)")
        .execute(&mut *lock)
        .await
        .expect("take migration advisory lock");
    let result = notification_infrastructure::migrations::run(&pool).await;
    let _ = sqlx::query("SELECT pg_advisory_unlock(9812462)")
        .execute(&mut *lock)
        .await;
    drop(lock);
    result.expect("run migrations in the disposable schema");

    (pool, schema)
}

/// Drops a disposable schema.
pub async fn drop_schema(schema: &str) {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let admin = PgPool::connect(&url)
        .await
        .expect("connect to test database");
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(&admin)
        .await
        .expect("drop disposable schema");
    admin.close().await;
}

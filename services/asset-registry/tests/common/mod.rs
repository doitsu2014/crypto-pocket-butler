//! Shared integration-test harness.
//!
//! Set `TEST_DATABASE_URL` to a disposable Postgres database. Each test gets a
//! unique schema, runs the real sqlx migrations into it, and drops it afterwards.
//! No test touches the `registry` schema or another service's schema.

#[allow(dead_code)]
pub mod fakes;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

#[allow(dead_code)]
pub async fn test_pool() -> (PgPool, String) {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must point at a disposable Postgres database for integration tests",
    );
    let schema = format!("test_registry_{}", Uuid::new_v4().simple());

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
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("connect disposable pool");

    asset_registry_infrastructure::migrations::run(&pool)
        .await
        .expect("run migrations in the disposable schema");

    (pool, schema)
}

#[allow(dead_code)]
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

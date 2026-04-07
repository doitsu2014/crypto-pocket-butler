---
name: rust-api
description: Axum-based Rust API endpoint development, SeaORM patterns, and OpenAPI documentation
---

# Rust API Development

## Context

Working with the Axum-based Rust API in `api/`.

## Conventions

- Entity models use SeaORM (see `api/src/entity/`)
- Route handlers organized in `api/src/handler/` or `api/src/routes/`
- Use utoipa derive macros for OpenAPI documentation
- JWT authentication via axum-keycloak-auth middleware
- Follow rustfmt and clippy recommendations

## Adding a New Endpoint

1. Create or update SeaORM entity in `api/src/entity/`
2. Create handler function in appropriate module under `api/src/handler/`
3. Add utoipa annotations with `#[utoipa::path(...)]`
4. Register route in the router
5. Add migration if schema change is needed
6. Write tests

## Common Patterns

- Shared state: `axum::extract::State<AppState>`
- JSON bodies: `axum::Json<T>` with serde deserialize
- Path params: `axum::extract::Path<T>`
- Query params: `axum::extract::Query<T>`
- Database access: `sea_orm::DatabaseConnection` from app state
- Error handling: convert domain errors into HTTP responses via `IntoResponse`

## Testing

```bash
cd api && cargo test
```

For integration tests that need a database, ensure PostgreSQL is running:
```bash
docker-compose up -d postgres
```

## Linting

```bash
cd api && cargo clippy -- -D warnings
cd api && cargo fmt --check
```

## API Documentation

Swagger UI is available at `http://localhost:3000/swagger-ui` when the API is running.
OpenAPI spec at `http://localhost:3000/api-docs/openapi.json`.

Use utoipa derive macros to document endpoints:
```rust
#[utoipa::path(
    get,
    path = "/api/v1/resource",
    responses(
        (status = 200, description = "Success", body = Vec<Resource>)
    ),
    security(("bearer_auth" = []))
)]
```

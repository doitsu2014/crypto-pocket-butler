---
name: database
description: PostgreSQL and SeaORM database migration, entity management, and schema conventions
---

# Database and Migrations

## Context

PostgreSQL database managed via SeaORM migrations in `api/migration/`.

## Migration Commands

```bash
cd api/migration
cargo run -- up      # Apply pending migrations
cargo run -- down    # Rollback last migration
cargo run -- reset   # Reset database (down + up)
```

## Adding a New Migration

1. Create new migration file in `api/migration/src/`
2. Implement `up()` method with schema changes
3. Implement `down()` method to reverse changes
4. Register the migration in the migration list
5. Run `cargo run -- up` to apply
6. Update or create the corresponding SeaORM entity in `api/src/entity/`

## Conventions

- All timestamps stored as UTC
- Use UUIDs for primary keys where appropriate
- Foreign keys with appropriate ON DELETE constraints
- Create indexes on frequently queried columns
- SeaORM entity files must match the current database schema

## SeaORM Entity Patterns

Entities live in `api/src/entity/`. Each entity file defines:
- The model struct with column mappings
- The relation definitions
- The active model behavior

Example entity structure:
```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "table_name")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}
```

## Database Connection

Default local connection:
- Host: `localhost:5432`
- Database: `crypto_pocket_butler`
- User: `postgres`
- Password: `postgres`

Connection string: `postgres://postgres:postgres@localhost/crypto_pocket_butler`

## Entity Generation

After running migrations, update SeaORM entities:
```bash
cd api && sea-orm-cli generate entity \
    -u postgres://postgres:postgres@localhost/crypto_pocket_butler \
    -o src/entity
```

Or manually update entity files to match the new schema.

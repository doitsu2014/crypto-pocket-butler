# Rust conventions

The house style, as it is actually practised in this tree. Copy the nearest existing example rather
than inventing a variant — consistency across twelve services is worth more than a locally nicer
shape.

## Where a thing goes

| Thing | Crate | Note |
|---|---|---|
| Entity, value type, id newtype, business rule | `domain` | no Axum, sqlx, reqwest, chrono-formatting or serde transport types |
| Use case, port trait, `AppError` | `application` | orchestrates; never imports `infrastructure` or `http` |
| Postgres repository, outbound HTTP client, Vault, JWKS, OTLP | `infrastructure` | implements a port defined inward |
| Router, middleware, handler, DTO, problem body | `http` | DTO ↔ domain mapping happens here and nowhere else |
| Config, wiring, `bin/{api,worker,migrate}.rs` | `services/<name>/src` | the composition root may depend on everything |

Id newtypes live in a `domain` `ids` module (`UserId`, `PortfolioId`, …) and are parsed once, at
the boundary, so an invalid id cannot travel inward as a bare string.

## Errors

- `domain::DomainError` names business refusals and knows nothing about HTTP.
- `application::AppError` is the use-case error. It is built only through `from_domain`,
  `from_repository` and `from_external` (`services/identity/application/src/common.rs` is the
  reference copy). Each service has its own; they are deliberately similar, not shared.
- `http` maps `AppError` to a status and an RFC 7807 body (see `problem-details.md`).
- Distinguish recoverable from fatal where the choice is made — retry with backoff for a transient
  dependency, fail fast for a broken invariant. Never `let _ = …` over a result that could fail.
- In `domain` and `application`, `unwrap()` does not appear on a production path: propagate with `?`.
  `expect` is acceptable only where a preceding line has proven the invariant, with the reason in
  the message.

## Numeric and temporal types

- `rust_decimal::Decimal` for every quantity, price, amount and percentage. Never `f32`/`f64` for
  money. Rounding is explicit — state the scale and the `RoundingStrategy` at the point of rounding,
  not implicitly.
- `chrono::DateTime<Utc>` for instants. A stored timestamp is UTC; formatting for a user happens in
  the frontend.

## Documentation in code

Every module opens with `//!` stating what it does and **which BR ids it serves**, e.g.

```rust
//! Order intents (BR5.1–BR5.3): advisory buy or sell intentions with a
//! projected allocation and drift, and an explicit shortfall when a sell
//! exceeds the holdings. No external order call exists.
```

Public items get a `///` doc comment; `# Errors` sections name the refusals a caller must handle.
When a choice is non-obvious — a relaxed constraint, an ordering that looks arbitrary — the comment
says why, not what.

## Naming

- Ports: `UserRepository`, `ShreddingKeyRepository`, `ConsentRepository` … (`Send + Sync` traits in
  `application/src/ports.rs`).
- Adapters: `Pg<User>Repository` for sqlx, plus the protocol in the name for HTTP clients.
- Migrations: `NNNN_<snake_name>.{up,down}.sql`, numbered consecutively within the service.
- Test files: `<layer>.rs` under the service's `tests/` (`domain.rs`, `application.rs`, `http.rs`,
  `repositories.rs`, `composition.rs`).

## Security-sensitive code

Any path that touches credentials, tokens, another user's data or a privileged action:

- enforces the ownership check in the same place the read/write happens;
- writes its audit event through the service's outbox in the same transaction;
- logs the actor, action, target, timestamp, context and outcome, and never a credential, token or
  full address.

## What a reviewer will look at first

Correctness and security before style (team practice). In this codebase that means: did the
boundary direction hold, is the ownership check present, is the transaction duration honest, is the
error mapped rather than unwrapped, and does the test file cover the happy path plus at least two
error or edge cases.
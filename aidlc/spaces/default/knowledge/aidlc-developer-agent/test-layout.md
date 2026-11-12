# Test layout

Where a test lives is decided by what it proves, not by what is convenient. This is the layout every
service follows; `services/identity` is the largest worked example.

## The files

```
services/<name>/tests/
├── common/
│   ├── mod.rs        shared harnesses, fixtures, claim builders, a fixed clock
│   └── fakes.rs      hand-written port fakes (in-memory repositories, stub gate, stub clock)
├── domain.rs         rules on entities and value types
├── application.rs    use cases against the fakes
├── http.rs           the Axum router, status codes and problem bodies
├── repositories.rs   sqlx adapters against a real schema  (#[ignore])
├── *_adapters.rs     outbound HTTP/Vault/JWKS adapters   (#[ignore] where they need Postgres)
└── composition.rs    the wiring the service package assembles
```

Some services keep suites inside the crates' own `tests/` directories instead; the rule is the same
— one file per layer, named for the layer.

## What each layer's tests assert

- **domain** — the rule, in isolation: legal transitions, refusals, rounding, ordering. No I/O, no
  fakes.
- **application** — the use case against port fakes: the outcome *and* the observable effects (the
  record written, the audit event emitted, the port not called on a refused path). A refusal path
  deserves a test as much as a happy path — most real defects live there.
- **http** — the router via `tower::ServiceExt::oneshot`, with no live port:
  `router(state.clone()).oneshot(request).await`. Assert the status and the RFC 7807 body,
  including `correlationId`. `services/identity/tests/http.rs` is the reference.
- **repositories / adapters** — against a real disposable Postgres (`TEST_DATABASE_URL`) or a stub
  server, `#[ignore]`d by default. For an HTTP adapter bind an ephemeral port
  (`tokio::net::TcpListener::bind(("127.0.0.1", 0))`) and assert **what the stub received** and how
  the response mapped, not that a line ran.

## The floor every test file carries

Happy path plus at least two error or edge cases. A test that would pass against any implementation
is a defect, not coverage. Where a behaviour is timing- or clock-shaped, inject a fixed clock
(`common::fixed_now`) instead of sleeping.

## The database gate

`#[ignore]` does not mean optional. Every gate run provisions a throwaway `supabase/postgres`
container, points `TEST_DATABASE_URL` at it, and runs the ignored suites as a **blocking** gate.
Those suites create and drop roles and apply and reverse migrations, so they must never see a
shared database. Run migration-bearing suites with `--test-threads=1` to avoid two suites migrating
the same schema at once.

## Naming

A test name states the behaviour, in the present tense, as a claim a reader can judge:
`an_active_user_cannot_be_anonymised`, `the_same_subject_hash_twice_is_refused`. Not `test_provision`
and not `works`. When it fails, the name alone should say what broke.

## Frontend tests

Vitest + Testing Library, colocated as `*.test.ts(x)` next to the component, plus shared MSW
handlers under `src/test/`. Assert through roles, labels and visible text; a test that reaches for a
CSS class is testing the wrong thing. See the quality knowledge for the command list.
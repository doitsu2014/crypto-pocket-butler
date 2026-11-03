---
name: clean-architecture
description: >
  Verify Clean Architecture crate boundaries in Crypto Pocket Butler Rust
  services: dependencies point inward only (http/infrastructure → application
  → domain), domain imports no framework, application never imports
  infrastructure or http, and DTOs never leak inward. Use when reviewing,
  refactoring, or adding dependencies to a Rust service, or before merging
  changes that touch any service's Cargo.toml or crate imports.
argument-hint: "[service path, e.g. services/portfolio — default: all services]"
---

# Verify Clean Architecture Boundaries

The dependency rule is enforced mechanically. This skill is the agent-side check; CI runs the
same logic on every build.

## The rule

```
http (interface)  →  application  →  domain
infrastructure    →  application  →  domain
```

Dependencies point inward only. Forbidden edges, any of which is a finding:

1. `domain` depending on a sibling crate (`application`, `infrastructure`, `http`).
2. `domain` depending on a framework: `axum`, `sqlx`, `reqwest`, `tokio`, `serde_json`,
   `tower`, or any HTTP/DB client crate.
3. `application` depending on `infrastructure` or `http`.
4. Any crate depending on `http`.
5. Inner crates (`domain`, `application`) containing DTO/transport types — check for
   `#[derive(Serialize, Deserialize)]` on request/response-shaped structs there.
6. `infrastructure` types appearing in `application` use-case signatures — dependencies cross
   boundaries only as ports (traits).

## The check

For each service workspace under `services/`:

```bash
# Edge 1+2: domain must have no sibling or framework deps
grep -E 'axum|sqlx|reqwest|tokio|tower|serde_json|^\s*(application|infrastructure|http)\s*=' \
  services/<name>/domain/Cargo.toml

# Edge 3: application must not depend on infrastructure or http
grep -E '^\s*(infrastructure|http)\s*=' services/<name>/application/Cargo.toml

# Edge 4: nothing may depend on http
grep -rlE '^\s*http\s*=' services/<name>/{domain,application,infrastructure}/Cargo.toml
```

Any match = a violation. Then confirm by reading the `use` statements where a Cargo.toml
match is ambiguous (a `domain` dep on `serde` with derive is allowed for persistence-shaped
value objects; `serde_json` and transport types are not).

## Fixing

Move the code inward, not the dependency outward: an infrastructure detail found in
`application` becomes a port (trait) in `application` or `domain`, implemented in
`infrastructure`. Never "fix" the check by whitelisting an edge.

Report findings as: crate, forbidden dependency or leaked type, and the inward-moving fix.

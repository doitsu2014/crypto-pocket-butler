# Reporting-domain Bugfix Test Instructions

## Framework

Rust/Cargo workspace. The affected package is `reporting-domain`; no test directory or test module was found in the focused source scan.

## Exact commands

- Runner readiness and crate build: `cargo check -p reporting-domain`
- Architecture regression gate: `bash scripts/check-boundaries.sh`
- If tests are added or discovered during implementation: run `cargo test -p reporting-domain`.

## Test and acceptance approach

- The boundary script is the targeted regression: before the fix it reports the forbidden serde dependencies; after the manifest cleanup the reporting-domain violation must be absent.
- Build the crate to establish dependency resolution and ensure no hidden source/macro dependency was missed.
- The full boundary script may still report unrelated existing violations in other crates. Record that result accurately; do not modify unrelated crates as part of this Unit.
- No database, migration, HTTP, frontend, or coverage setup is required for this manifest-only change.

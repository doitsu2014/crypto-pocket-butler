# Build Instructions — Reporting-domain Dependency Fix

## Prerequisites

- Rust toolchain and Cargo for the workspace.
- No environment variables, secrets, database, or frontend toolchain are required for this manifest-only change.

## Build and architecture checks

From the workspace root, run:

```bash
cargo check -p reporting-domain
bash scripts/check-boundaries.sh
```

The Cargo check verifies dependency resolution and compilation. The boundary script verifies that no forbidden framework dependency remains in domain crates. The focused regression suite is `cargo test -p reporting-domain`.

## Inputs and verification

The build scope and allowed files are defined in `../code-generation/code-generation-plan.md`. The exact crate-scoped commands and acceptance approach are in `../code-generation/unit-test-instructions.md`; implementation details and prior test evidence are in `../code-generation/code-summary.md`. Build and test results for this run are recorded in `test-results.md`. These instructions do not provision or connect to any shared or production database.
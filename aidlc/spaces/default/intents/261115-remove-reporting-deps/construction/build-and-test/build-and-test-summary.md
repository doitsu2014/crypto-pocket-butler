# Build and Test Summary — Reporting-domain Dependency Fix

## Overall status

Build and targeted verification passed. The targeted regression check is green, all 46 `reporting-domain` tests pass, and no runtime behavior or schema changes are involved.

## Test instruction inventory

- Build instructions: `build-instructions.md` — `cargo check -p reporting-domain` and `bash scripts/check-boundaries.sh`.
- Additional test-instruction files: none. Minimal strategy applies.
- Executed focused tests: `cargo test -p reporting-domain` — 46 passed, 0 failed.
- Not run: workspace-wide tests (not required by this manifest-only plan); integration, performance, security, browser, and database-backed checks are not applicable.

## Coverage expectations

The affirmed target is at least 80% Rust line coverage per crate. Command: `cargo llvm-cov -p reporting-domain -p reporting --ignore-run-fail --summary-only`. The reporting-domain source modules total 1,093 lines, with 1,024 executed (93.69%). Each of the six reported domain modules also exceeds 80%. The paired command also measures the reporting service package; its 73.12% combined total is not the reporting-domain result.

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| FR1 / AC1 | Requirements FR1, AC1 | No serde dependencies in reporting-domain | Both declarations absent | `services/reporting/domain/Cargo.toml` | Build and Test | Met |
| FR2 / AC2 | Requirements FR2, AC2 | Other reporting layers retain required dependencies | Sibling manifests unchanged; checks passed | Scoped diff; Code Generation review | Build and Test | Met |
| FR3 / AC3 | Requirements FR3, AC3 | Crate compiles and tests pass | Compile passed; 46 tests passed | Cargo command results in `test-results.md` | Build and Test | Met |
| FR4 / AC4 | Requirements FR4, AC4 | Boundary violation absent | 60-package check passed | `bash scripts/check-boundaries.sh` | Build and Test | Met |
| NFR1 | Requirements NFR1 | Architecture boundary passes without exception | Passed; no exception or allowlist added | `bash scripts/check-boundaries.sh` | Build and Test | Met |
| NFR2 / AC5 | Requirements NFR2, AC5 | Minimal, behavior-preserving change | Only the intended manifest and lockfile dependency edges changed | Scoped diff; Code Generation summary | Build and Test | Met |
| Rust line coverage | Team Testing Posture | At least 80% per crate | reporting-domain: 1,024 / 1,093 lines (93.69%) | `cargo llvm-cov -p reporting-domain -p reporting --ignore-run-fail --summary-only` | Build and Test | Met |

## Readiness

- Build-ready: Yes.
- Test-ready: Yes for the approved manifest-only scope.
- Deployment-ready: Not applicable; this change has no deployment or schema component.

## Limitations

The full workspace test suite and Postgres-backed ignored suites were not run. No database was used. No frontend, API, migration, or runtime behavior changed.
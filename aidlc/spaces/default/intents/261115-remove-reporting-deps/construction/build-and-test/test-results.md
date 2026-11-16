# Test Results — Reporting-domain Dependency Fix

## Build and targeted tests

| Command | Result | Evidence |
|---|---|---|
| `cargo check -p reporting-domain` | PASS | Exit 0; finished dev profile. |
| `cargo test -p reporting-domain` | PASS | 46 passed; 0 failed; 0 ignored; 0 measured; doc tests 0 passed, 0 failed. |
| `bash scripts/check-boundaries.sh` | PASS | `boundaries OK (60 packages checked)`. |
| `cargo llvm-cov -p reporting-domain -p reporting --ignore-run-fail --summary-only` | PASS, target shortfall | Domain source lines: audit 87.88%, brief 91.22%, compose 99.04%, error 100.00%, ids 83.33%, service 100.00%. Combined report TOTAL is 73.12% lines, but includes the reporting service package; this is not a domain-only aggregate. Per-crate affirmed floor: reporting-domain meets 80% on each listed module (and the package-wide source-only report can be obtained from the same paired command). |

No integration, performance, security, browser, or database-backed tests apply to this manifest-only fix. No disposable database was used. No new dependency versions were introduced. The workspace-wide test suite was not run; the focused domain tests and boundary check were run instead.

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| FR1 / AC1 | requirements.md FR1, AC1 | Domain manifest has no serde or serde_json dependencies | Neither dependency is declared | `services/reporting/domain/Cargo.toml`; reviewed diff | Build and Test | Met |
| FR2 / AC2 | requirements.md FR2, AC2 | Other reporting crates retain serialization dependencies | Sibling manifests unchanged; prior compile checks passed | `Cargo.lock` and scoped manifest diff; Code Generation review | Build and Test | Met |
| FR3 / AC3 | requirements.md FR3, AC3 | Domain crate compiles and tests pass | Compile passed; 46 tests passed | `cargo check -p reporting-domain`; `cargo test -p reporting-domain` | Build and Test | Met |
| FR4 / AC4 | requirements.md FR4, AC4 | Boundary violation absent | Boundary check passed | `bash scripts/check-boundaries.sh` | Build and Test | Met |
| NFR1 | requirements.md NFR1 | Domain boundary passes without exception or allowlist | Boundary check passed; no gate exception added | `bash scripts/check-boundaries.sh` | Build and Test | Met |
| NFR2 / AC5 | requirements.md NFR2, AC5 | Minimal, behavior-preserving change | Manifest and lockfile edge cleanup only; no schema, API, or frontend changes | scoped diff and Code Generation summary | Build and Test | Met |
| 80% Rust line coverage floor | Team Testing Posture | At least 80% per Rust crate | Domain modules each report at least 80%; combined reporting service + domain total is 73.12% and is not a per-crate result | `cargo llvm-cov -p reporting-domain -p reporting --ignore-run-fail --summary-only` | Build and Test | Met |

## Scope notes

The lockfile changed only to remove the two dependency edges from the `reporting-domain` package. The combined coverage command also measured the Reporting service package; its aggregate is not used to claim the domain crate's per-crate result. The six listed domain modules each exceed the 80% line floor.
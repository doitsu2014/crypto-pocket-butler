# Requirements — Remove Unused Reporting Domain Dependencies

**Intent:** remove unused serialization dependencies from `reporting-domain` so the Clean Architecture boundary check passes.

**Source:** approved intent description and focused Reverse Engineering scan (`inception/reverse-engineering/developer-scan.md`). The scan read `services/reporting/domain/Cargo.toml` and every source module in that crate. It found that `serde` and `serde_json` are declared but not used by the reporting-domain source; `scripts/check-boundaries.sh` explicitly forbids both dependencies in domain crates.

## Intent analysis

The reporting domain crate contains two manifest-only dependencies that violate the repository's enforced dependency boundary. Removing these unused declarations should restore the domain crate's framework-free dependency rule without changing runtime behavior or unrelated reporting service crates.

## Functional requirements

| ID | Requirement | Source | Priority |
|---|---|---|---|
| FR1 | `reporting-domain` shall not declare `serde` or `serde_json` as dependencies when its source does not use either crate. | Focused scan: `services/reporting/domain/Cargo.toml` and `services/reporting/domain/src/`; project Clean Architecture rule | Must |
| FR2 | The fix shall not remove or alter serialization dependencies in reporting application, infrastructure, HTTP, or service crates where source code uses them. | Focused workspace search; request scope | Must |
| FR3 | After the manifest change, the reporting domain crate shall compile and its tests shall pass. | Project build/test commands | Must |
| FR4 | The repository Clean Architecture boundary check shall pass for `reporting-domain`. | `scripts/check-boundaries.sh`; project boundary rule | Must |

## Acceptance criteria

- AC1 — Given the reporting domain manifest currently declares `serde` and `serde_json` but scanned source has no usage, when the manifest is updated, then neither dependency is listed in `services/reporting/domain/Cargo.toml`.
- AC2 — Given other reporting crates use serde serialization, when this change is applied, then their manifest dependencies and source remain unchanged.
- AC3 — Given the dependency declarations are removed, when the reporting domain crate is built and tested, then the relevant crate tests pass.
- AC4 — Given the dependency declarations are removed, when `bash scripts/check-boundaries.sh` runs, then the reporting-domain dependency-boundary violation is absent.
- AC5 — Given this is a manifest-only cleanup with no schema or runtime behavior change, when the change is reviewed, then no migration, new service, API contract, or frontend change is included.

## Non-functional requirements

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR1 | Preserve the Clean Architecture rule that domain crates have no framework or transport dependencies. | `bash scripts/check-boundaries.sh` passes for all domain crates; no exception or allowlist is added. | Project architecture rule and boundary script |
| NFR2 | Keep the change minimal and behavior-preserving. | Only the unused dependency declarations in `services/reporting/domain/Cargo.toml` are removed; no unrelated code or lockfile churn beyond required Cargo resolution. | User request and scan evidence |

## Constraints

- Rust workspace package management remains Cargo; do not substitute tools or add dependencies.
- Keep `reporting-domain` free of serialization/framework dependencies.
- Do not remove serde/serde_json from reporting application, infrastructure, HTTP, or service crates; those layers use serialization.
- No schema, API, deployment, or runtime behavior change is expected; therefore no migration is planned.
- Do not weaken or bypass `scripts/check-boundaries.sh`.

## Assumptions

| # | Assumption | Rationale | Owner |
|---|---|---|---|
| A1 | Removing the two unused manifest entries is sufficient; no generated code or macro expansion in the domain crate depends on them. | Focused scan found no imports or derives, and verification will compile the crate. | Developer, during Code Generation |

## Out of scope

- Changing serialization usage in reporting application, infrastructure, HTTP, or service crates.
- Refactoring reporting domain models or changing serialized API/database representations.
- Adding CI configuration, new infrastructure, or a new test framework.
- Fixing unrelated boundary failures in other crates.

## Traceability summary

| Need | Requirement / criterion |
|---|---|
| Remove unused domain dependencies | FR1; AC1 |
| Preserve dependencies actually used by other reporting layers | FR2; AC2 |
| Keep crate behavior buildable | FR3; AC3 |
| Restore enforced architecture boundary | FR4; AC4; NFR1 |
| Keep scope minimal | AC5; NFR2 |

# Code Summary — Remove unused reporting-domain serialization dependencies

**Intent:** `261115-remove-reporting-deps`
**Unit scope:** `reporting-domain` manifest-only cleanup (FR1–FR4, NFR1, NFR2)
**Builds:** remove the two unused Clean Architecture-forbidden dependencies from `reporting-domain`

## Files created / modified

| File | Change |
|---|---|
| `services/reporting/domain/Cargo.toml` | Removed `serde.workspace = true` and `serde_json.workspace = true` from `[dependencies]`. No other line touched. |
| `Cargo.lock` | Two lines removed from the `reporting-domain` package's dependency list (`"serde"`, `"serde_json"`). Cargo produced this automatically during `cargo check`; no other lockfile entry changed. |

No source file, sibling manifest, migration, schema, API, frontend, CI or unrelated crate was
changed. Sibling `serde` declarations remain intact, verified by grep:

- `services/reporting/Cargo.toml` (composition root) — still declares both
- `services/reporting/application/Cargo.toml` — still declares both
- `services/reporting/infrastructure/Cargo.toml` — still declares both
- `services/reporting/http/Cargo.toml` — still declares both

`services/reporting/domain/src/` contains no `serde` or `serde_json` reference at all — the grep
over the whole domain crate matched only the two manifest lines that were removed.

## Key decisions

1. **Manifest-only fix, no source change.** The reverse-engineering scan and a fresh grep both
   found no import, derive or macro use of `serde`/`serde_json` anywhere under
   `services/reporting/domain/src/`. Removing the declarations therefore cannot change runtime
   behaviour or serialized representations.
2. **`Cargo.lock` accepted rather than reverted.** The plan allowed `Cargo.lock` to change only if
   Cargo proved it must. `cargo check -p reporting-domain` rewrote the `reporting-domain` package's
   dependency list in the lock file (a two-line deletion). That is the required, truthful
   resolution of the manifest, not incidental churn, so the lock change is kept. Nothing else in
   the lock file moved.
3. **The boundary script is the regression test.** For a defect that lives entirely in a manifest,
   the narrowest level that reproduces it is the mechanical boundary gate, which is exactly how the
   defect was found. No Rust test was added because no unit-test seam exists for a dependency
   declaration: a test asserting "serde is not in Cargo.toml" would restate the manifest rather
   than exercise behaviour, which the construction guardrails rule out ("never generate tests that
   always pass regardless of implementation"). The gate reaching a red state and then a green state
   is the TDD red/green pair for this unit, and the existing 46 domain tests are the
   behaviour-preservation check.

## Verification — exact commands and real results

Run in this order from the worktree root
`C:\Users\Admin\orca\workspaces\crypto-pocket-butler\feat-authentication-session-management`.

### Step 1 — Runner readiness

Command: `cargo check -p reporting-domain`
Result: **PASS** (exit 0). Output ended `Finished \`dev\` profile ... in 5.73s`. This was run
before the manifest edit and confirmed the crate compiled with the two dependencies still declared
(the baseline). The orchestrator had also verified this step before dispatch.

### Step 2 — Regression Red

Command: `bash scripts/check-boundaries.sh`
Result: **FAIL (exit 1)** — the expected red state, and the exact defect:

```
boundary check FAILED:
  - reporting-domain must not import framework crates: ['serde', 'serde_json']
```

One violation only; no other crate failed, so the blast radius is the single manifest.

### Step 3 — Green

Manifest edit applied: the two `serde`/`serde_json` lines removed from
`services/reporting/domain/Cargo.toml`.

Command: `cargo check -p reporting-domain`
Result: **PASS** (exit 0), `Finished \`dev\` profile ... in 5.73s`. Confirms no hidden source or
macro dependency on either crate (assumption A1 in the requirements held).

Command: `bash scripts/check-boundaries.sh`
Result: **PASS** (exit 0) — `boundaries OK (60 packages checked)`. The `reporting-domain`
violation is gone and no new violation appeared.

Command: `cargo test -p reporting-domain`
Result: **PASS** (exit 0) — `46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`, and the
doc-test harness `0 passed; 0 failed`. Behaviour preserved across all domain modules
(`audit`, `brief`, `compose`, `error`, `ids`, `service`).

Command: `cargo check -p reporting-application -p reporting-infrastructure -p reporting-http -p reporting`
Result: **PASS** (exit 0), `Finished \`dev\` profile ... in 19.56s`. The sibling crates that do use
serde still resolve and compile.

Command: `cargo clippy -p reporting-domain --all-targets -- -D warnings`
Result: **PASS** (exit 0) — no warnings, so the `-D warnings` gate is clean for this crate.

Command: `cargo fmt -p reporting-domain -- --check`
Result: **PASS** (exit 0, no output). Formatting unaffected.

### Step 4 — Diff inspection

Commands: `git status --porcelain -- services/reporting Cargo.lock`, `git diff --stat -- services/reporting Cargo.lock`, `git diff -- services/reporting/domain/Cargo.toml Cargo.lock`

Result: exactly two files changed, four deletions total, no additions:

```
 Cargo.lock                           | 2 --
 services/reporting/domain/Cargo.toml | 2 --
 2 files changed, 4 deletions(-)
```

The `Cargo.lock` hunk removes only `"serde"` and `"serde_json"` from the `reporting-domain`
package block. Sibling reporting crates are untouched. (Git warned that `Cargo.lock` will get CRLF
line endings on the next touch — a pre-existing Windows checkout artifact, not a change made here.)

### Step 5 — Traceability

Command: `aidlc engine sensor-traceability --stage code-generation --output-path aidlc/spaces/default/intents/261115-remove-reporting-deps/construction/code-generation/traceability.json --write-upstream`
Result: scaffold written with upstream ids `["FR1","FR2","FR3","FR4","NFR1","NFR2"]`.

The rows were then filled, and the verification re-run without `--write-upstream`:

Command: `aidlc engine sensor-traceability --stage code-generation --output-path aidlc/spaces/default/intents/261115-remove-reporting-deps/construction/code-generation/traceability.json`
Result: **`{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}`**

Note on the target column: this sensor requires every `OK` target to be exactly one existing
workspace-relative *file path* and rejects prose, so each row names the single file that carries
the evidence (`services/reporting/domain/Cargo.toml`, `scripts/check-boundaries.sh`,
`Cargo.lock`, `services/reporting/domain/src/lib.rs`, `services/reporting/application/Cargo.toml`)
rather than carrying a sentence. The reasoning that would otherwise sit beside the path is in this
summary instead.

## Tests written but not executed

None. No test file was added, so nothing was left unrun. The reasoning for adding no Rust test is
recorded under Key decisions above.

## Checks that could not run, and why

- **`cargo test --workspace`** — not run. It is not in this unit's approved plan or unit-test
  instructions, and it needs the disposable `supabase/postgres:15.8.1.060` database up with
  `TEST_DATABASE_URL` set for the `#[ignore]` adapter and migration suites. Those suites are
  Postgres adapter tests in other services, untouched by a `reporting-domain` manifest edit. The
  crate-scoped suite and the four sibling crate checks were run instead.
- **`cargo llvm-cov` coverage reading** — not run, and deliberately so. Nothing executable in this
  unit changed; a coverage figure would measure unmodified source, and the team's runbook forbids
  quoting one without its invocation anyway. No coverage number is claimed here.
- **Frontend `yarn test` / `yarn lint` / `yarn build` / `yarn e2e` / `yarn size`** — not run. No
  TypeScript, frontend or auth-surface file was touched, so the auth-surface `yarn e2e` gate does
  not apply to this change.
- **Dependency/container scanning (`cargo audit`, Trivy)** — not run. No new or changed dependency
  version exists: `serde` and `serde_json` were already in the workspace and are still declared
  everywhere they are used, so the lock file lost two edges and gained none.

## Traceability

Every requirement in `inception/requirements-analysis/requirements.md` is covered `OK`:

| ID | Evidence |
|---|---|
| FR1 | `services/reporting/domain/Cargo.toml` no longer lists `serde` or `serde_json`; a grep over the whole crate finds no reference. |
| FR2 | `services/reporting/{application,infrastructure,http}/Cargo.toml` and `services/reporting/Cargo.toml` still declare both; `git diff` shows no change to them; the four sibling packages all compile. |
| FR3 | `cargo check -p reporting-domain` passed; `cargo test -p reporting-domain` 46 passed, 0 failed. |
| FR4 | `bash scripts/check-boundaries.sh` reports `boundaries OK (60 packages checked)`; the `reporting-domain` violation is absent. |
| NFR1 | The boundary check passes for all domain crates with no exception or allowlist added — `scripts/check-boundaries.sh` is unmodified. |
| NFR2 | The diff is four deletions across two files: two manifest lines plus the two matching lock entries. No behaviour change, no schema change, no migration, no API or frontend change. |

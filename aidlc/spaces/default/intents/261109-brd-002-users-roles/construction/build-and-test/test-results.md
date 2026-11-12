# Build and Test Results

Run: 2026-11-12T05:54:34Z, re-measured on the loop-back replay 2026-11-12T07:44Z–07:49Z ·
Intent `261109-brd-002-users-roles` · Stage Build and Test
(Standard depth, Standard test strategy) · assembled project at the workspace root.

Every command below was re-run on the replay against the repaired tree; the numbers in §1–§4 are
the replay's, and §3 keeps the original diagnosis of each failure with the replay's status.

Commands were run once each, deduplicated from `build-instructions.md`,
`integration-test-instructions.md` and the ten per-Unit `unit-test-instructions.md`
files. The exit code recorded for every row is the exit code of the command itself
(captured without a pipe), not of a reporting step after it.

## 1. Build status

| Command | Exit | Output |
|---|---|---|
| `cargo build --workspace` | 0 | workspace built clean (debug profile) |
| `yarn --cwd web build` (`tsc -b && vite build`) | 0 | bundle emitted to `web/dist` |
| `yarn --cwd support-console build` | 0 | bundle emitted to `support-console/dist` |
| `bash scripts/check-boundaries.sh` | 1 | `reporting-domain must not import framework crates: ['serde', 'serde_json']` |
| `cargo fmt --all -- --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | no warnings |

## 2. Test results

| Command | Exit | Total | Passed | Failed | Ignored / filtered |
|---|---|---|---|---|---|
| `cargo test --workspace` | 0 | 1 813 | 1 659 | 0 | 154 ignored |
| `TEST_DATABASE_URL=… cargo test --workspace -- --ignored --test-threads=1` | 101 | 92 | 91 | 1 | 3 filtered (`identity` repository suite) |
| `yarn --cwd web test` (`vitest run`) | 0 | 312 | 312 | 0 | 0 (42 files) |
| `yarn --cwd support-console test` (`vitest run`) | 0 | 272 | 272 | 0 | 0 (38 files) |
| `yarn --cwd web lint` (ESLint 9) | 0 | — | — | — | — |
| `yarn --cwd support-console lint` | 0 | — | — | — | — |
| `yarn --cwd web format:check` (Prettier 3) | 1 | 81 files differ | — | 81 | — |
| `yarn --cwd support-console format:check` | 1 | 11 files differ | — | 11 | — |
| `yarn --cwd web size` (size-limit) | 0 | 29 chunks | 29 within budget | 0 | initial 101.2 kB gzip across 2 files; 27 route chunks ≤ 150 kB |

The blocking database gate ran against a disposable `supabase/postgres:15.8.1.060`
container (`cpb-bt-test-pg`, port 55433). Two environment facts had to be fixed before
it could run and are part of the evidence:

1. The connecting role must be a member of the roles the migrations create. Connecting
   as `postgres` (not a superuser and not a member of `identity_app` / `audit_app`)
   made `SET ROLE` fail, which surfaced as three `PoolTimedOut` failures in the audit
   suite. Pointing `TEST_DATABASE_URL` at `supabase_admin` — the image's superuser —
   cleared those three.
2. The first attempt ran against a database reused from an earlier Unit run. The gate
   was re-run against a freshly created database (`identity_test2`) so no stale schema
   could mask or fabricate a result.

On the replay the gate ran against a second freshly created database (`identity_test3`) in the
same disposable container, so no stale schema from the first run could mask or fabricate a
result. One failure remains (F2b); it reproduces on both fresh databases, so it is a defect in the
guarantee, not test state. The first run's second failure (F2a) is fixed — see §3.

### Browser behaviour

No target in this run was verified in a real browser engine. The two frontend suites
run in jsdom (a DOM emulator); the assembled bundles were built but never driven by
Playwright, Cypress or any other real browser. Wherever this run reports browser
behaviour, it is jsdom evidence, which cannot show native browser behaviour.

## 3. Failure details

### F1 — Boundary gate: `reporting-domain` declares framework crates

```
$ bash scripts/check-boundaries.sh
reporting-domain must not import framework crates: ['serde', 'serde_json']
$ echo $?
1
```

`services/reporting/domain/Cargo.toml:13-14` declares `serde.workspace = true` and
`serde_json.workspace = true`; nothing under `services/reporting/domain/` references
either crate (the only occurrence of the string `serde` in that crate is the two
manifest lines). The crate is unmodified in the working tree and was last touched by
commit `6544576` (2026-11-09), which landed before this intent's Units, so this is a
pre-existing project gate failure outside the ten Units of this intent.

### F2 — `services/identity/tests/repositories.rs` (1 failed, 35 passed, exit 101 on the replay; 2 failed, 28 passed on the first run)

These are `#[ignore]`d seam tests that only run when `TEST_DATABASE_URL` points at a
pgsodium-capable disposable database. Neither was flaky: both failed identically on a
freshly created database on the first run.

**F2a — `a_user_is_found_by_the_hashed_subject_and_the_subject_is_protected_at_rest` — fixed on the replay**

```
thread '…' panicked at services/identity/tests/repositories.rs:115:34:
called `Result::unwrap()` on an `Err` value: Conflict(
    "the account is no longer pending deletion; the erasure was refused")
```

The test stores an **Active** user (`stored_user` builds `User::provision`, status
`active`) and then calls `users.anonymise(&user)`. `PgUserRepository::anonymise`
(`services/identity/infrastructure/src/repositories/users.rs:351-406`) deliberately
guards the tombstone move with `WHERE id = $1 AND status IN ('pending_deletion',
'deleted')` so a concurrent restore wins and an account restored mid-erasure is never
silently erased. That guard is the contract the account-lifecycle Unit also tests
(`services/identity/tests/account_lifecycle_adapters.rs:466`, which seeds
`pending_deletion`). The U1 test therefore asserts the pre-U5 behaviour: it is stale
relative to the erasure rule that landed in U5.

**Repair, replayed and verified.** U1's Loop-back 1 repair drives the stored user to
`pending_deletion` through `User::transition` plus `UserRepository::update` before erasing, and
adds a case asserting that `anonymise` on an active user is refused with `Conflict`. The replay's
suite reports 35 passed, 1 failed — the remaining failure is F2b.

**F2b — `protect_and_unprotect_are_security_definer_functions_the_app_may_execute`**

```
thread '…' panicked at services/identity/tests/repositories.rs:392:5:
a destroyed key no longer decrypts the subject
```

The test creates a shredding key, protects a value as `identity_app`, destroys the key
through `PgShreddingKeyRepository::destroy`, and then expects `unprotect(...)` to fail.
It does not. `destroy_shredding_key` (migration `0001_init.up.sql:292-301`) only runs
`DELETE FROM pgsodium.key WHERE key_id = target_key_id`; the row is deleted (key ids
skip 20 afterwards), but pgsodium 3.1.8 keeps the derived key material in a
server-wide cache. Verified outside the test suite: in a brand-new session against the
same server, after the row was gone,

```
SET ROLE identity_app;
SELECT length(protect('probe', 20)) > 0;   -- → t
```

So a destroyed key still encrypts and decrypts until the cache is evicted (server
restart). pgsodium 3.1.8 exposes no destroy/flush function, and `validate`-style
workarounds do not exist. The crypto-shredding guarantee as written (key destruction
makes the subject unrecoverable *immediately*) is not delivered by the mechanism; the
ciphertext deletion that `anonymise` performs in the same transaction is intact and is
what actually removes the stored subject.

### F3 — Prettier: `web` and `support-console`

`yarn --cwd web format:check` → `Code style issues found in 81 files`; 41 of them are
source files and 40 are generated `web/dist/**` bundles (gitignored, included because
the package has no `.prettierignore`). `yarn --cwd support-console format:check` →
`Code style issues found in 11 files`.

Against the working tree, 35 of the 41 web source files and all 11 console files are
**unchanged by this intent** (`git status --porcelain` reports nothing for them), so
the drift pre-dates this work. The six intent-touched files that differ are:

- `web/src/api/settingsQueries.test.tsx`
- `web/src/features/account-status/AccountBlockedPage.tsx`
- `web/src/features/settings/PreferencesPanel.test.tsx`
- `web/src/features/settings/PreferencesPanel.tsx`
- `web/src/features/signup-gates/useGateStatus.test.tsx`
- `web/src/features/signup-gates/useGateStatus.ts`

Reformatting only those six leaves the command red (35 pre-existing web files + 11
console files + the `dist` set), so the gate cannot be turned green inside this
intent's remit.

### F4 — Structurizr model validation: `Unverified`

`docs/architecture/workspace.dsl` is modified by this intent, and the project rule is
to validate the Structurizr model before review (`docs/architecture/README.md` §12).
Two image/environment attempts were made:

| Attempt | Result |
|---|---|
| `docker run --rm -v …:/data structurizr/cli:latest validate -w /data/workspace.dsl` | CLI prints only its deprecation banner and exits 0 without validating |
| `docker run --rm -v …:/data structurizr/structurizr:latest validate -w /data/workspace.dsl` | exit 1: `ERROR … The content from https://static.structurizr.com/themes/default/theme.json could not be loaded: static.structurizr.com: Name does not resolve` |

The consolidated image does validate, but it resolves the default theme over the
network and this machine's DNS cannot reach `static.structurizr.com`. `validate` accepts
no theme override (`usage: validate -w <arg>`), so the model cannot be checked offline
here. The in-stage attempts are exhausted: `Unverified`.

### F5 — CI obligation: `N/A`

The stage-level and contract instruction files carry "run the selected tests in CI
before merge". `aidlc-state.md` lists `ci-pipeline` as `SKIP` and `performance-validation`
as `SKIP`, so no CI job exists in this plan and none is scheduled later. The target is
`N/A` with that reason: it is neither a deferral to a later stage nor a failure.

### F6 — Coverage floor: five touched crates below 80 % lines

On the replay's measurement against the 80 % per-crate line floor (Interview Q4),
`identity-infrastructure` 64.95 %, `gateway-infrastructure` 68.72 %, `account-http` 79.35 %,
`portfolio-http` 45.38 % and the `gateway` composition root 3.23 % are below the floor. The
`identity` composition root moved from 0.00 % to 84.65 % under U1's repair and now meets it.
Numbers and denominators are in §4. The adapters' gaps are error and retry branches the database
suites do not reach; `identity-infrastructure`'s remaining gap is spread over `auth.rs` (134
uncovered lines), `support/drainer.rs` (90), `support/sources.rs` (89), `status_transitions.rs`
(82), `contributors.rs` (54), `support/observation.rs` (42) and `repositories/deletions.rs` (42);
`portfolio-http`'s gap is 465 uncovered lines in the Portfolio service's own handlers
(`handlers.rs` 54/519) and worker (`worker.rs` 0/90), code this intent touched only through the
support-metadata seam.

### F7 — Cross-Unit traceability: 17 ids uncovered

`aidlc engine sensor-traceability --stage build-and-test` returns `pass: false`. Full breakdown in
`cross-unit-traceability.md`; the four causes:

| Cause | Ids | Owning Unit |
|---|---|---|
| Claimed in `upstream_ids` but no coverage row points at a file | AC6.4.1, AC6.4.2 (the code exists at `principal.rs:176`) | `u6-jurisdiction-gate` |
| Claimed by no Unit at all | AC8.1.1 | none — fell out at Units Generation |
| Deferred with no later stage in this plan to own it | AC5.1.1–AC5.3.2 and FR18.1–18.3 (Account/Portfolio storage, outside this intent); AC4.4.2, FR17.4, FR19.4 (legal hold); NFR26.2 (console localisation) | none in this plan |
| Malformed evidence file | NFR23.1, NFR23.2, NFR23.3 (target is not a workspace-relative path) | `u6-jurisdiction-gate` |

The U6 ids are an evidence defect, not missing code: the gate already clears before any personal
data is written. The thirteen deferred ids are a scope fact — they belong to services and Units
this intent does not build — and the sensor still counts every deferral as uncovered, which is the
sensor's rule and was not relaxed.

## 4. Coverage report

Measured once on the assembled project with the project's chosen tool
(`cargo llvm-cov`, Interview Q4) and the ignored database suites included, because a
floor of 80% line coverage is set per Rust crate and the adapter suites are the only
coverage those crates get:

```
$ TEST_DATABASE_URL=postgres://…/identity_test3 \
    cargo llvm-cov --workspace --ignore-run-fail --json \
    --output-path target/coverage.json -- --include-ignored --test-threads=1
```

`cargo llvm-cov` exit 0; the underlying `cargo test` reported 91 passed / 1 failed
(exit 101) and `--ignore-run-fail` was used so the report is still written — the failure is F2b.

Workspace total: **70.99 % lines** (37 584 / 52 942 covered), 560 files.

| Crate | Lines covered | Coverage | ≥ 80 % floor | Touched by this intent |
|---|---|---|---|---|
| `identity-domain` | 2 106 / 2 315 | 90.97 % | Met | yes |
| `identity-application` | 3 174 / 3 513 | 90.35 % | Met | yes |
| `identity-infrastructure` | 1 716 / 2 642 | 64.95 % | **Not Met** | yes |
| `identity-http` | 2 925 / 3 636 | 80.45 % | Met | yes |
| `identity` (composition root) | 623 / 736 | 84.65 % | Met | yes |
| `gateway-domain` | 232 / 250 | 92.80 % | Met | yes |
| `gateway-application` | 174 / 190 | 91.58 % | Met | yes |
| `gateway-infrastructure` | 145 / 211 | 68.72 % | **Not Met** | yes |
| `gateway-http` | 358 / 427 | 83.84 % | Met | yes |
| `gateway` (composition root) | 3 / 93 | 3.23 % | **Not Met** | yes |
| `account-domain` | 1 163 / 1 264 | 92.01 % | Met | yes |
| `account-application` | 1 549 / 1 755 | 88.26 % | Met | yes |
| `account-http` | 780 / 983 | 79.35 % | **Not Met** | yes |
| `portfolio-domain` | 1 276 / 1 407 | 90.69 % | Met | yes |
| `portfolio-http` | 629 / 1 386 | 45.38 % | **Not Met** | yes |

Untouched neighbouring crates are listed for context only and are not judged here:
`account-infrastructure` 24.25 %, `portfolio-application` 84.66 %,
`portfolio-infrastructure` 17.17 %.

Every met crate clears the floor with the database suites included. The five crates
still below the floor are: `identity-infrastructure` (64.95 %, up from 53.63 % on the repair),
`gateway-infrastructure` (68.72 %), the `gateway` composition root (3.23 %, 90 uncovered lines in
`config.rs` 30 and `lib.rs` 57), and the two HTTP crates touched through the support-metadata seam
(`account-http` 79.35 % — 7 lines short of 787, `portfolio-http` 45.38 %). None of these was
relaxed or rounded up.

## 5. Final Target Verification Matrix

The authoritative per-target matrix, with actual values and evidence for every target,
is `build-and-test-summary.md` §4; its verdicts are final for this run. Summary of
outcomes:

| Target group | Verdict | Cause |
|---|---|---|
| Builds (BT-B1, BT-B2, BT-B3) | Met | all three exit 0 |
| Formatting (BT-Q1), tests (BT-Q4), frontend tests/lint/build/size (BT-F1, BT-F2, BT-F4, BT-F5, BT-F6) | Met | exit 0 with the counts in §2 |
| Boundary gate (BT-Q3) | Not Met | F1 |
| Blocking database gate (BT-Q5) | Not Met | F2b (F2a fixed on the replay) |
| Clippy (BT-Q2) | Met | exit 0, no warnings |
| Coverage floor (BT-Q6, BT-CV1–CV12) | Not Met | §4: five touched crates sit below 80 % |
| Frontend formatting (BT-F3, BT-F7) | Not Met | F3 |
| Accessibility (BT-A1) | Met | `jest-axe` assertions in both suites pass |
| Contract obligations (BT-T1) | Met | unit-level contract assertions pass |
| Contract obligations (BT-T2, TDD write order) | Unverified | write order is not observable in history; one Unit's summary overstated its Red/Green record |
| CI (BT-C1) | N/A | F5 |
| Structurizr model (BT-D1) | Unverified | F4 |
| Per-Unit test targets, suites (`-S`) | Met for U2–U6, U8–U10; Not Met for U1 and U7 | both `-S` rows include the Identity repository suite, which now fails 1 of 36 (F2b) |
| Per-Unit requirement targets (`-N`) | Met for U1, U3, U4, U7, U8; Not Met for U6; Unverified for U2, U5, U9, U10 | `cross-unit-traceability.md` §3–§5 |
| Cross-Unit traceability (Step 10) | **fail** | 17 ids uncovered: 2 claimed but untraced (U6), 1 unclaimed (AC8.1.1), 13 deferred outside this intent or with no owning stage, 3 malformed targets in U6's evidence file |
| Browser-only checks (BT-X1) | Unverified for native behaviour | jsdom only, see §2 |

## 6. Failure escalation

Escalation follows the stage's ladder.

1. **In-stage fixes (rung 1, two attempts used).** Environment setup for the blocking
   database gate: the connecting role was corrected to the image superuser, and the
   gate was re-run against a freshly created database. This cleared three audit-suite
   `PoolTimedOut` failures and confirmed the two identity failures are real. The
   Structurizr attempts are recorded as F4.
2. **Classify and estimate impact (rung 2).** F1, F3 and F4 have root causes outside
   this intent's generated code: a manifest in the reporting service landed by commit
   `6544576`, frontend formatting drift on files this intent never touched, and a DNS
   blockade for the model validator. F2a's root cause is generated test code (a U1 seam
   test written before the U5 erasure rule). F2b's root cause is the shredding design
   in generated code together with a pgsodium behaviour the design assumed differently.
   F6's root cause is generated production code with untested error branches, plus two
   binary entry points no test drives. F7's root cause is an evidence defect in U6 (two
   acceptance criteria satisfied by code that has no coverage row, and three malformed
   targets), plus one id no Unit ever claimed and thirteen ids that belong to services this
   intent does not build.
3. **Rung 3 was not entered** because the bound would be spent on work that cannot
   turn the stage green while F1, F3 and F4 remain: no loop-back into a Unit of this
   intent can remove a pre-existing failure in `services/reporting`, reformat 46 files
   this work never touched, or give the validator network access. F7's evidence
   defect *would* be loop-back-able (it is entirely inside U6), but the other
   thirteen uncovered ids are a scope boundary this stage cannot move, and a
   loop-back that leaves F1, F3 and F4 red cannot turn the stage green, so the
   bound is better spent after the person decides how to handle the out-of-remit
   failures.
4. **Halt-and-ask (rung 4).** The failures and the estimated impact of each candidate
   fix are put to the person; the stage's approval gate is not presented on this run.

**Replay status (2026-11-12T07:44Z–07:49Z).** On the loop-back replay the ladder was applied
again to the re-measured result, and it lands one rung earlier than the first run did:

- **Fixed by the repair:** F2a (the stale U1 seam test) and the `identity` composition root
  (0.00 % → 84.65 %), so BT-Q5 now fails on F2b alone and BT-CV5 is `Met`.
- **Reduced but still red:** `identity-infrastructure` 53.63 % → 64.95 % (floor needs 2 114
  covered lines; 1 716 are covered).
- **Unchanged:** F1, F3, F4, F2b, F7, and the coverage gaps in `gateway` (3.23 %),
  `gateway-infrastructure` (68.72 %), `account-http` (79.35 %, 7 lines short) and
  `portfolio-http` (45.38 %).
- **Rung 3 continues the loop-back already open.** Loop-back 1's planned fix named three
  remaining legs that have not been executed yet — U1's coverage work
  (`identity-infrastructure`, the `gateway` root and `gateway-infrastructure`), U3's
  (`account-http`, `portfolio-http`, gateway `upstream`) and U6's evidence file (two coverage
  rows, NFR23 targets). Executing them does not spend a second entry of the bound; this replay
  is the same failed run's forward path.
- **Still outside every Unit:** F2b (pgsodium 3.1.8 caches destroyed key material — no code
  change makes the immediate-unrecoverability assertion true), F1 (`reporting-domain`, last
  touched before this intent), F3 (46 pre-existing source files plus generated `dist`), F4 (this
  machine's DNS), AC8.1.1 (claimed by no Unit) and the thirteen ids deferred with no owning stage.
  The person's decision is required for those; this stage cannot make them green.

## 7. Evidence index

- `build-instructions.md` — prerequisites, build commands, troubleshooting.
- `integration-test-instructions.md` — seam evidence, database gate recipe.
- `build-and-test-summary.md` §4 — the authoritative Target Verification Matrix.
- `cross-unit-traceability.md` — the id-level upstream coverage verdict (F7).
- Per-Unit `code-generation/code-summary.md` — per-Unit Red/Green evidence and known gaps.

## Loop-Back Log

Bound: 3 entries per intent. This section is append-only; a later run never edits an earlier
entry. The count of `### Loop-back N` entries is the bound.

### Loop-back 1 — 2026-11-12T06:21:47Z

**Diagnosis.** This run failed on five fronts (F1, F2, F3, F4, F6, F7). Three of them have their
root cause in code generated by Units of this intent: F2a, a seam test in
`services/identity/tests/repositories.rs` that stores an active user and then calls `anonymise`,
which the erasure guard deliberately refuses; F7, eleven requirement ids that `u2-authorisation`
and `u6-jurisdiction-gate` claim in their `upstream_ids` but never implement (FR18.1–18.3,
AC5.1.1–AC5.3.2, AC6.4.1, AC6.4.2) plus three malformed NFR23 targets in U6's evidence file;
and F6, six touched crates under the 80 % line floor whose untested paths are adapter error
branches, the two composition roots, and the Account/Portfolio HTTP support-metadata seams.

**Root-cause stage.** `code-generation`.

**Planned fix.**

| Unit | Fix |
|---|---|
| `u1-user-registry` | Drive the seam test to `pending_deletion` before calling `anonymise`, so it asserts the U5 erasure guard instead of the pre-U5 path; add the adapter and entry-point tests that lift `identity-infrastructure` and the `identity` root. |
| `u2-authorisation` | Implement FR18.1–18.3 and AC5.1.1–AC5.3.2 in `services/identity/application/src/authorisation/`, with tests, and record them in `traceability.json`. |
| `u3-support-access` | Add the tests that cover the Account and Portfolio HTTP support-metadata seams (`account-http`, `portfolio-http`) and the gateway `upstream` paths. |
| `u6-jurisdiction-gate` | Implement AC6.4.1 and AC6.4.2 with tests; rewrite NFR23.1–NFR23.3 as workspace-relative file paths. |

**Estimated impact.** Effort: generation across four Units, the largest item being the two new
authorisation rules and their six acceptance criteria; risk: the new rules sit on a security
boundary and must pass the reviewer before this stage's gate; the coverage additions are test-only
in three of the four crates.

**Not fixed by this loop-back, and why.**

- **F2b** (a destroyed pgsodium key still decrypts): pgsodium 3.1.8 exposes no cache flush, so no
  code change can make the "unrecoverable immediately" assertion true. The assertion stays red;
  the guarantee's wording — not the test — has to be decided.
- **F1** (`reporting-domain` serde), **F3** (formatting drift on files this intent never touched),
  **F4** (validator DNS): root causes live outside every Unit of this intent.
- **AC8.1.1**: claimed by no Unit; it needs a plan decision, not a repair to an existing Unit.
- **AC4.4.2 / FR17.4 / FR19.4 / NFR26.2**: deferred with no later stage owning them; a repair to
  an existing Unit cannot supply them.

**Re-run condition.** On the forward replay, Build and Test re-executes Step 9 with the Artifact
Re-use choice set to **Modify** for this stage, so this log survives.

**Correction to the diagnosis above (appended in the same run, nothing removed).** Re-reading the
Units' evidence files changed two points. FR18.1–18.3 and AC5.1.1–AC5.3.2 are **not** unbuilt work
in U2: U2's `traceability.json` marks all six AC5 ids `Deferred`, with a reason each, because the
portfolio-link storage they describe belongs to the Account and Portfolio services, which this
intent does not build. And AC6.4.1/AC6.4.2 are satisfied by code that already exists —
`services/identity/application/src/principal.rs:176-178` clears the jurisdiction gate before any
personal data is written — but U6 has no coverage row pointing at it. So the U2 leg of this
loop-back is withdrawn, and the U6 leg is an evidence-file repair (add the two coverage rows, fix
the three NFR23 targets), not new behaviour. What remains genuinely in scope is U1 (F2a, F6) and
U3 (F6).
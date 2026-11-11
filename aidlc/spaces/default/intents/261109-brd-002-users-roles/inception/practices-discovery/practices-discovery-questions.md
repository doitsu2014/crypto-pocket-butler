# Practices Discovery — Interview Questions

The evidence review re-confirmed most of the team's standing practices: trunk-based work on `main`, the thin end-to-end slice, test-first development, production-only Docker Swarm, and the Clean Architecture code style. These questions cover only what the repository contradicts or cannot establish.

## Q1. [release engineer] Rolling updates: keep the stack's start-first policy or move to the literal Swarm defaults?

The affirmed rule says releases roll out "using the Swarm defaults", but `infra/swarm-stack.yml` actually configures `order: start-first`, `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`. One of the two has to give.

- A. Keep the stack as it is and update the rule to match it (`start-first`, `parallelism: 1`, `failure_action: rollback`, 30s monitor)
- B. Change the stack to the literal Swarm defaults (`stop-first`, pause on failure)
- C. Drop the mechanics from the rule and state only that a service's health check gates its update
- X. Other (please specify)

[Answer]: A — Keep the stack as it is and update the rule to match it (`start-first`, `parallelism: 1`, `failure_action: rollback`, 30s monitor).

## Q2. [release engineer] Yarn version: correct the rule to the pinned 4.18.1?

The project rule says "Yarn ≥ 6", but every `package.json` pins `yarn@4.18.1` through corepack. No Yarn release satisfies "≥ 6", so the rule as written cannot be followed.

- A. Correct the rule to "Yarn 4.18.1, pinned through corepack"
- B. Keep "Yarn ≥ 6" and upgrade the pin to a matching release
- C. Drop the version number from the rule and say only "Yarn pinned through corepack"
- X. Other (please specify)

[Answer]: B — Keep "Yarn ≥ 6" and upgrade the pin to a matching release. (Superseded by Q8: no Yarn 6 exists, so the rule is corrected to Yarn 4.18.1.)

## Q3. [release engineer] CI platform: leave it open, or choose one now?

There is no CI in the repository today — no `.github/`, GitLab, Jenkins, or CircleCI configuration — so the documented build, lint, test, boundary, coverage and scanning gates all depend on a person running them.

- A. Leave the CI platform open for now and record it as an intention
- B. GitHub Actions
- C. GitLab CI
- D. Jenkins or CircleCI
- X. Other (please specify)

[Answer]: A — Leave the CI platform open for now and record it as an intention.

## Q4. [quality engineer] Coverage tooling: how should the 80% per-crate target be measured?

The 80% line-coverage-per-Rust-crate target is affirmed, but nothing in the repository measures it: no tarpaulin, llvm-cov or nextest configuration, and no coverage job.

- A. `cargo llvm-cov` (source-based, integrates with rustfmt/clippy tooling)
- B. `cargo tarpaulin`
- C. `cargo nextest` plus `llvm-cov`
- D. Leave coverage tooling open for now
- X. Other (please specify)

[Answer]: A — `cargo llvm-cov`.

## Q5. [quality engineer] Integration-test database: how should the six ignored suites run?

Six test suites carry `#[ignore]` because they need a disposable Postgres database reached through `TEST_DATABASE_URL`. Nothing provisions that database or runs the ignored suites today, so the adapter integrations are effectively untested in any automated run.

- A. Provision a disposable Postgres per run and run the ignored suites as a blocking gate
- B. Keep them opt-in: a developer sets `TEST_DATABASE_URL` when they want them
- C. Run them against the local Docker Compose stack on demand
- D. Leave the approach open for now
- X. Other (please specify)

[Answer]: A — Provision a disposable Postgres per run and run the ignored suites as a blocking gate.

## Q6. [release engineer] Down migrations on rollback: build the automatic step, or narrow the rule?

The rule says a rollback applies the matching down migration automatically, and every schema change ships an up/down pair — but nothing in the repository ever applies a `.down.sql` file. The `migrate` binaries apply up migrations only.

- A. Keep the rule and build the automatic down step as part of the release job
- B. Narrow the rule: rollbacks are forward fixes only; down migrations are kept for manual use
- C. Leave the rollback mechanics open for now
- X. Other (please specify)

[Answer]: A — Keep the rule and build the automatic down step as part of the release job.

## Q7. [delivery lead] Canonical build/lint/test commands: where do they live?

`aidlc/spaces/default/memory/project.md` still carries placeholder `Build`, `Lint` and `Test` fields. The real commands live in the per-package Yarn scripts and in `docs/development/README.md`, and the Rust commands appear only in the rules text.

- A. Fill the placeholders in `aidlc/spaces/default/memory/project.md` and make it the single authoritative list
- B. Keep `docs/development/README.md` authoritative and leave the memory file's fields as pointers to it
- C. Rely on the package scripts only
- D. All three, with `aidlc/spaces/default/memory/project.md` winning on conflict
- X. Other (please specify)

[Answer]: A — Fill the placeholders in `aidlc/spaces/default/memory/project.md` and make it the single authoritative list.

## Q8. [release engineer] Yarn "≥ 6" cannot be met by upgrading — which way should the rule go?

Yarn's published release line today is 1.x, 2.x, 3.x and 4.x; there is no Yarn 5 or 6 release to upgrade the pin to, so Q2's "keep ≥ 6 and upgrade" cannot be carried out. The rule and the pin still have to agree.

- A. Keep "Yarn ≥ 6" exactly as written as the team's rule, with the pin staying at 4.18.1 until a 6.x release exists
- B. Correct the rule to "Yarn 4.18.1, pinned through corepack"
- C. Drop the version number from the rule and say only "Yarn pinned through corepack"
- X. Other (please specify)

[Answer]: B — Correct the rule to "Yarn 4.18.1, pinned through corepack". (Resolves Q2: keep the rule and pin in agreement at 4.18.1.)

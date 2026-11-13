# Coverage runbook

The affirmed target is **80 % line coverage per Rust crate**, measured with `cargo llvm-cov`. This
file exists because measuring it wrongly is easy and the wrong number has already sent one repair
round after the wrong thing.

## The measurement trap

`cargo llvm-cov -p <crate>` instruments **only that package**. In this repository most tests live in
the *service* package (`services/<name>/tests/*.rs`), not in the library crate they exercise — so a
per-package run leaves the library's lines uncovered and reports a number far below reality.

Two ways to get the true number:

```bash
# Whole workspace, every test including the #[ignore] database suites
cargo llvm-cov --workspace --include-ignored --ignore-run-fail \
  --json --output-path target/coverage.json -- --test-threads=1

# One crate, paired with the package that carries its tests
cargo llvm-cov -p identity-infrastructure -p identity --include-ignored --ignore-run-fail
```

The paired form is the one to use while iterating on a single crate; the workspace form is the one
to quote in a results table. Record **which command produced the number** next to it — a coverage
figure without its command is not evidence.

## Read coverage after the ignored suites, not before

The `#[ignore]` adapter and migration suites are a blocking gate, not an opt-in. If they did not
run, every repository line reads as uncovered and the crate looks catastrophically worse than it is.
`--include-ignored` with `TEST_DATABASE_URL` set is the only valid reading.

## The disposable database

The suites need a throwaway PostgreSQL built from the image the project pins
(`supabase/postgres:15.8.1.060`), because Identity migrations and adapter cases use `pgsodium`. The
shape used today:

```bash
docker run --rm -d --name cpb-gate-db \
  -e POSTGRES_PASSWORD=<throwaway> -p 5432:5432 supabase/postgres:15.8.1.060

export TEST_DATABASE_URL='postgres://postgres:<throwaway>@127.0.0.1:5432/postgres'
```

The connection must be a superuser: the suites create and drop roles. Never point it at a shared or
production database. There is no committed provisioning script yet — that is a known gap, and
whoever writes it should keep this exact contract (pinned image, superuser, one database per run).

## Reading a shortfall

- Report it with the numbers and the uncovered files, per crate, in the build-and-test results table.
- **Never lower the target or redefine the metric** to make a step pass. A crate at 64.95 % is
  reported as 64.95 %, with the modules that account for the gap named.
- Prefer tests that assert behaviour over tests that merely execute lines. A module at 100 % with
  assertions that cannot fail is worse evidence than one at 70 % with real ones.
- Held-red tests are legitimate evidence: a test that fails because of a genuine library limitation
  (pgsodium 3.1.8 caching destroyed key material, for instance) is reported as failing with the
  reason, not deleted to make the suite green.

## TypeScript

No coverage threshold is set. The frontend gates are `yarn test`, `yarn lint`, `yarn build` and
`yarn format:check` in both `web/` and `support-console/`; `web/` additionally runs `yarn size`
against size-limit budgets (300 kB initial entry chunk, 150 kB route chunk, 150 kB stylesheet, all
gzip). Do not introduce a coverage threshold as a side effect of another change.

## What a quality summary carries

Per crate: covered / total lines, percentage, and the command used. Per unit: which acceptance
criteria have a witness in code and which do not. Where a criterion is unclaimed, say so — a
traceability table full of `OK` rows that cite the wrong file is worse than an honest gap.
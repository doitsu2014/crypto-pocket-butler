# Practices Discovery — Questions

The draft in `team-practices.md` and `discovered-rules.md` is drawn from the project's own
documents. These questions settle the points those documents leave open. Answer each by
writing a letter after `[Answer]:` (for example `[Answer]: A`), or pick `X` and say what you
want instead.

## Q1. [Delivery lead] How we branch and merge

The repository already states trunk-based development with short-lived branches, a human review
before merge, and every pull request linking to its requirement or unit id. Should that stand as
the team's way of working?

- A. Keep it exactly as drafted — trunk-based, short-lived branches, every PR links a requirement or unit id, human review before merge
- B. Keep it, and add a maximum branch age (for example two working days) and small-PR guidance
- C. Move to a release-branch model with a protected `main`
- X. Other (please specify)

[Answer]: A

## Q2. [Delivery lead] Build a thin end-to-end slice first?

A walking skeleton is a minimal version that runs the whole way through, built first to prove
the pieces connect before the real features go in. Should we build one?

- A. Yes — start with the Asset Registry slice: SPA shell → API Gateway → Asset Registry service → its own Postgres schema → staging Docker Swarm, with health checks and observability
- B. Yes, but start with a different service (say which one)
- C. No — go straight to feature breadth
- X. Other (please specify)

[Answer]: A

## Q3. [Quality engineer] Testing cadence

The project requires requirement-driven tests, a happy-path floor per component, and
Given/When/Then acceptance criteria, but it does not say whether tests come before or after the
code. Which cadence should the team follow?

- A. Custom, as drafted — Given/When/Then acceptance criteria before code, then application use-case tests against port fakes and adapter integration tests in the same change as the code
- B. Test-first (TDD) for all production code
- C. Test-after — write tests once the implementation works
- X. Other (please specify)

[Answer]: B

## Q4. [Quality engineer] Coverage threshold

No document states a numeric coverage target today. Should there be one?

- A. No numeric threshold — require the happy path plus at least two error or edge cases per test file, and judge business-rule coverage
- B. 80% line coverage per crate
- C. 90% line coverage per crate
- X. Other (please specify)

[Answer]: B

## Q5. [Release engineer] Environments and promotion

The C4 model describes one Production environment on a single Swarm cluster, while the scope
document adds staging and a three-manager production cluster. How should releases be promoted?

- A. Staging → production, with a manual approval before production
- B. Production only, no separate staging environment
- C. Staging → production automatically when the checks are green
- X. Other (please specify)

[Answer]: B

## Q6. [Release engineer] How a release rolls out

Docker Swarm's default update behaviour is the current starting point. Which release strategy
should the team use?

- A. Rolling updates with Swarm defaults, health-checked
- B. Blue-green per service, with a smoke test before switching traffic
- C. Canary release with a traffic percentage and abort criteria
- X. Other (please specify)

[Answer]: A

## Q7. [Release engineer] Database migration policy

Each service owns its own Supabase Postgres schema, but the sqlx migration policy is not yet
stated. How should schema changes ship?

- A. Forward-only sqlx migrations run as a pre-deploy step, with a documented rollback plan per release
- B. Versioned up/down migrations with an automatic down step on rollback
- C. Decide at the first service rather than now
- X. Other (please specify)

[Answer]: B

## Q8. [Developer] Lint and format tooling

The documents require the same lint and test commands locally as in CI, but do not name the
tools. Which should the team standardise on, and with which commands?

- A. `rustfmt` + `clippy` for Rust (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`); ESLint + Prettier through Yarn for TypeScript
- B. `rustfmt` + `clippy` for Rust; Biome for TypeScript
- C. No enforced formatter for now — decide when the first code lands
- X. Other (please specify)

[Answer]: A

## Q9. [Release engineer] Staging contradicts the scope document — which governs?

You chose Production only, but the ideation scope document
(`ideation/scope-definition/scope-document.md`) lists a staging environment and a
three-manager production cluster as launch gates C11–C12. One of them has to give.

- A. Production only — drop the staging launch gate and record the scope change
- B. Keep staging — staging → production with a manual approval before production
- C. Production only at launch; add staging afterwards
- X. Other (please specify)

[Answer]: A

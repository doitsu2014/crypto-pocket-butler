# Evidence — Practices Discovery

**Project:** Crypto Pocket Butler · **Intent:** `261105-brd-001-product`
**Project type:** Greenfield — the repository carries business requirements, architecture
documentation, engineering rules, and a design system, but no application code.
**Stage:** `inception/practices-discovery`, Step 5 (lead integration).
**Collaborators:** off for this scope — no support-agent contributions were dispatched. The
lead draft from Step 2 was the only pre-interview input, and the human interview is the
authority on every point the documents left open.

## Sources inspected

| Source | What it is | What it supports |
| --- | --- | --- |
| `.claude/aidlc-common/stages/inception/practices-discovery.md` | Stage definition | Required artifacts, five section names, `Methodology`/`Ordering` fields, promotion flow |
| `aidlc/spaces/default/memory/org.md` | Organisation defaults | Human review before merge; testable acceptance criteria; no secrets in source or logs; walking skeleton is scope-dependent; requirement-driven tests with a happy-path floor |
| `aidlc/spaces/default/memory/team.md` | Current affirmed baseline (generic template) | Trunk-based development with short-lived branches; PR links to requirement/unit id; correctness and security before style; local and CI commands match; actionable alerts |
| `aidlc/spaces/default/memory/project.md` | Project memory (template placeholders) | Why-before-what commit messages; one unit of work per change. Name, stack, build, lint, and test fields are still `replace with ...` placeholders |
| `aidlc/spaces/default/memory/phases/inception.md` | Inception guardrails | Given/When/Then acceptance criteria; at least two alternatives per architecture decision; ADR fields; traceability; no unresolved contradictions |
| `aidlc/spaces/default/memory/phases/construction.md` | Construction guardrails | Complete runnable files; error handling at boundaries; no silent failures; happy path plus two error/edge cases; no hardcoded secrets; boundary input validation |
| `aidlc/spaces/default/memory/phases/operation.md` | Operation guardrails | Rollback steps; production smoke/health check; quantified SLOs; alerts below SLO breach; one health and one error-rate metric per service; runbook escalation; P1/P2 reviews |
| `CLAUDE.md` and `.claude/CLAUDE.md` | Harness onboarding | Confirms this is an AI-DLC workspace; no engineering practices of its own |
| `.claude/rules/development.md` | Ambient development rules | Fixed Rust/React stack; Clean Architecture dependency rule; schema/role isolation; Yarn ≥ 6; Docker Swarm; Vault; no message queues; gateway-only SPA calls; testing shape |
| `docs/development/README.md` | Development standards (authoritative detail) | Same stack and boundary rules; mechanical crate-boundary check in CI; test shape per component |
| `docs/architecture/README.md` | C4 model of record (13 Rust services incl. gateway, SPA, support console, observability stack) | Single `Production` Swarm environment; Supabase Auth/Postgres/Storage/Vault/Cron; JWT forwarding; no queues; append-only audit role; per-service schemas; decisions and accepted costs; model/README must change together |
| `docs/brds/README.md` | BRD index and conventions | MVP boundary; ID conventions (`BR-`, `FR-`, `NFR-`, `AC-`); MoSCoW; document-change discipline |
| `docs/brds/BRD-012-security-compliance-and-non-functional-requirements.md` | Security, privacy, legal, audit, NFRs | Read-only by default; least privilege; no secret exposure; Vault; no private keys; read-only exchange keys; MFA; JWKS; in-memory tokens; rate limits; audit fields; dependency/image scanning; TLS; 99.9% availability; RPO/RTO; SEV-1 timing |
| `design-system/DESIGN.md` | Parchment Ledger design system | Tokens only; typography and figure rules; no colour-only status; em dash for missing values; accessibility baseline; responsive rules |
| `.gitignore` | Repository hygiene | Generated Structurizr artefacts excluded; AI-DLC local cursors and runtime excluded |
| `git log --oneline -20`, `git log --merges`, `git branch -a` | Repository history and branches | Squash-merged `(#NNN)` pull requests on `main`; no merge commits; short-lived `chore/*` and `copilot/*` branches |
| Repository scan (root, `find` for build/CI/deploy files) | Files present | `design-system/`, `docs/`, `tools/`, `aidlc/`, `.claude/`, `CLAUDE.md`, `.gitignore`; **no** `Cargo.toml`, `package.json`, `yarn.lock`, `Dockerfile`, `docker-compose*`, `Makefile`, `rust-toolchain*`, `.github/workflows/`, `.gitlab-ci.yml`, `Jenkinsfile`, `.circleci/`, `infra/`, or `deploy/` |
| `tools/preview-architecture.sh` | Only executable tooling present | Docker-based Structurizr Local preview of `docs/architecture/workspace.dsl`; not a build, test, or deploy script |
| Ideation `scope-definition/scope-document.md` | Scope baseline (Option B MVP) | Ten launch services plus gateway, SPA, platform and observability; staging; three-manager Swarm; IaC-only; digest-pinned scanned images; CDN/WAF; rollback; SBOM; PITR; launch gates C1–C16 |
| `inception/practices-discovery/practices-discovery-questions.md` | Completed interview | The nine governing answers recorded below |

## Inferences (pre-interview)

- **Way of Working** — Trunk-based with short-lived branches was the only branch strategy named
  in memory, and the history is consistent with it: squash-merged pull requests on `main` and
  no merge commits. Conventional-commit prefixes appear in older history (`feat:`, `chore:`)
  but were not stated as a rule, so the draft did not mandate them. The interview confirmed the
  draft unchanged and did not add a convention.
- **Walking Skeleton** — The org default (thinnest end-to-end slice) plus the architecture's
  gateway/JWT/schema rules defined the shape of the slice. The choice of Asset Registry as the
  first service was an inference from its lack of external credential dependencies; the
  interview confirmed it.
- **Testing Posture** — The documents combined three signals: requirement-driven tests with a
  happy-path floor, Given/When/Then acceptance criteria, and use-case/adapter test placement.
  No document said "write the test first", so the honest draft was `custom`. The interview
  overrode this with TDD for all production code, which is the single largest change from the
  draft.
- **Deployment** — Docker Swarm and Vault were explicit. Staging, the three-manager cluster,
  IaC-only, digest-pinned images, and rollback came from the ideation scope document rather
  than the architecture model, which models `Production` only. The interview removed staging
  entirely, aligning the practice with the architecture model.
- **Code Style** — Rust crate boundaries, decimal arithmetic, React/TypeScript stack, tokens,
  and error handling were explicit. Formatter, linter, and the exact lint commands were absent;
  the interview supplied them. Test commands and naming conventions remain absent.

## Interview decisions

| # | Area | Answer | Effect on the artifacts |
| --- | --- | --- | --- |
| Q1 | Way of working | A — keep exactly as drafted | Trunk-based, short-lived branches, PR linked to a requirement or unit id, one human review before merge. No branch-age cap, PR size limit, branch naming pattern, commit-message convention, or CODEOWNERS rule added. `Way of Working` keeps its draft content; the open points about approvals and commit format are closed as "no additional rule". |
| Q2 | Walking skeleton | A — build it first, starting with Asset Registry | `Walking Skeleton` now records the slice as a decision, not a suggestion: SPA shell → API Gateway → Asset Registry service → its own Postgres schema → the Docker Swarm, with health checks and observability. The target is the production Swarm, because Q5/Q9 removed staging. |
| Q3 | Testing cadence | B — TDD for all production code | `- **Methodology**: custom` replaced with `tdd`; `Methodology evidence` rewritten to say the team affirmed test-first while the existing requirement-driven and BDD shape still governs what tests cover; `Ordering` rewritten to "a failing test first for every piece of production code, then the implementation, in the same change". Added as a mandated and a forbidden rule. |
| Q4 | Coverage | B — 80% line coverage per crate | Recorded as 80% line coverage per Rust crate, enforced in CI. The happy-path-plus-two-errors floor stays. Coverage tool and the TypeScript threshold were not asked and remain open. |
| Q5 | Environments | B — production only | `Environments` now states production only on the Docker Swarm; no staging tier and no promotion step. The draft's staging mention is removed from `Walking Skeleton` and `Deployment`. |
| Q6 | Release strategy | A — rolling updates with Swarm defaults, health-checked | `Deployment strategy` settled for every component; no blue-green or canary. Recorded as a mandated rule and as a health-check forbidden rule. |
| Q7 | Migrations | B — versioned up/down sqlx migrations, automatic down step on rollback | `Database migrations` settled at the policy level; the mechanics (who runs them, how the down step triggers, naming/ordering) remain open. Recorded as a mandated rule and as a down-migration forbidden rule. |
| Q8 | Lint and format | A — rustfmt + clippy; ESLint + Prettier through Yarn | Exact Rust commands recorded verbatim: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`. TypeScript tools named; the Yarn script names remain open. |
| Q9 | Staging contradiction | A — production only; drop the staging launch gate | Recorded as a scope change: the ideation scope document's C11–C12 staging gate no longer applies. The scope document itself is outside this stage's artifacts and still carries the old gate, so the change record is listed as an open action below. |

### Points the answers closed that the draft had left open

- Number of approvals and reviewer rules: one human review, no CODEOWNERS rule (Q1).
- Branch naming and commit-message format: none mandated beyond why-before-what (Q1).
- Test methodology and ordering: TDD, tests first (Q3).
- Numeric coverage: 80% per Rust crate (Q4).
- Environment topology and promotion: production only, no promotion (Q5, Q9).
- Deployment strategy: rolling with Swarm defaults (Q6).
- Migration policy: versioned up/down with automatic down on rollback (Q7).
- Rust and TypeScript lint/format tooling and the Rust commands (Q8).

## Remaining uncertainty

1. **CI platform and ordered gate list** — no CI configuration exists in the repository; the
   platform (GitHub Actions or otherwise) and the order of build, lint, test, boundary check,
   SAST, DAST, secret scan, SBOM, image scan, deploy, and smoke steps are not chosen.
2. **Test commands and runners** — Rust (`cargo test` vs `cargo nextest`) and TypeScript
   (Vitest, Jest, Playwright) are not named, and the Yarn script names for lint, format, and
   test are not fixed.
3. **Coverage tooling** — the 80% per-crate threshold is set but the tool that measures it
   (tarpaulin, llvm-cov, or other) is not, and no TypeScript coverage threshold was stated.
4. **Contract testing** — no approach or tool for API or cross-service contracts.
5. **Integration-test database provisioning** — how a real Postgres schema and its test data are
   provisioned per integration test.
6. **Migration mechanics** — who runs sqlx migrations in the pipeline, how the automatic down
   step is triggered on rollback, and migration naming and ordering.
7. **Release cadence and versioning scheme** — continuous, scheduled, semver, git SHA, or build
   number is not stated.
8. **Rollback triggers and approval path** — a failed post-update health check is the only
   stated trigger; the wider trigger set and who approves a production rollback are open.
9. **Feature flags** — whether flags gate deployments or remain a product-only capability.
10. **Naming conventions and pre-commit hooks** — beyond framework defaults, and whether a local
    check suite is required.
11. **Where authoritative commands live** — `memory/project.md` still carries placeholder build,
    lint, and test fields; whether the canonical commands live there, in
    `docs/development/README.md`, or both, and which wins on conflict.
12. **Rule enforcement scope** — whether every rule in `discovered-rules.md` is enforced
    mechanically or only an agreed subset.
13. **Scope document change record** — the ideation scope document still lists staging and the
    C11–C12 staging launch gate. The interview decision supersedes them, but the scope document
    has not been amended; that edit belongs to the scope owner, not this stage's artifacts.

## Residual risk

Production is the only environment, so the walking skeleton's first end-to-end deploy is a
production deploy. The safety net is the health-checked rolling update, the automatic down
migration on rollback, and the rollback path — none of which has been exercised yet, because no
application code or deployment tooling exists. The first release should treat the rollback path
as a deliverable to test, not an assumption.

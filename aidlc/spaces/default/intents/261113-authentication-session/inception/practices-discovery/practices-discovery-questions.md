# Practices Discovery — Questions

The draft practices are in `team-practices.md`; the evidence behind each suggested answer is in
`evidence.md`. These questions cover only the points the evidence could not settle — everything
else in the draft is already re-confirmed against the tree at commit `112b354`.

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [delivery lead] Branch lifetime and stale branches

Our recorded practice is "short-lived branches, merge back within days", but 96 of our 106 remote
branches were last touched between February and May 2026 and are still on the remote. This intent
also works on `feat/authentication-session-management`, which will live for the whole intent rather
than one unit. What should the practice say?

- A. Allow an intent's own branch to live as long as the intent, keep "merge within days" for unit
  work, and add no cleanup rule (treat the 96 old branches as dormant history)
- B. Same as A, plus a cleanup rule: delete remote branches untouched for 90 days
- C. No intent-branch exception — all branches resolve within days, including this one
- D. Leave the current wording as it is and treat this branch as ad hoc
- X. Other (please specify)

[Answer]: A — an intent's own branch may live as long as the intent; unit work still merges within days; no cleanup rule for the 96 dormant branches. **Mode:** guided

## Q2. [architect] What "done" means for a slice, and the gateway route rule

The gateway's route table is deny-by-default, and several paths the SPA already calls are not
declared there (`/signup-gate`, `/mfa/enrol`, `/mfa/recovery`, the device-session routes). Should
we make "every SPA call has a declared gateway route and an owning service handler" a standing
rule, and treat reachability through those declared routes as the completion test for a slice?

- A. Yes to both — make the rule mandated and the route table the completion test
- B. Yes to the rule, but no general completion test
- C. No new rule; fix the missing routes as ordinary work in this intent
- D. Leave it to the delivery plan without writing a rule
- X. Other (please specify)

[Answer]: A — make the rule mandated, and treat reachability through declared gateway routes as the completion test for a slice. **Mode:** guided

## Q3. [quality engineer] Does the browser journey suite gate a change?

`web/` has a Playwright suite (8 journeys over sign-in, dashboard and settings) that needs no
database or credentials, and it caught a first-render crash every unit test passed over. Should a
green `yarn e2e` gate a change?

- A. Gate any change touching the auth/sign-in surface (recommended)
- B. Gate every frontend change
- C. Keep it diagnostic only; never gate
- D. Gate only before a release
- X. Other (please specify)

[Answer]: A — a green `yarn e2e` gates any change touching the auth/sign-in surface. **Mode:** guided

## Q4. [quality engineer] How the 80% coverage floor is measured

A plain `cargo llvm-cov -p <crate>` under-reports badly here because most tests live in the service
package, and that has already cost a repair round. Should the floor stay at 80% per Rust crate with
the measurement contract written beside it?

- A. Yes — 80% per crate, measured with the workspace or paired `cargo llvm-cov` run, and the
  command quoted wherever a figure appears
- B. Keep the 80% but leave the command choice to whoever runs it
- C. Lower the floor for this intent
- D. Drop the per-crate floor and measure per service
- X. Other (please specify)

[Answer]: A — 80% per Rust crate, measured with the workspace or paired `cargo llvm-cov` run, with the command quoted beside every figure. **Mode:** guided

## Q5. [release engineer] CI platform and the ordered gate list

There is still no CI platform in the repository; the only pipeline in history was a Kubernetes
pipeline added in February and deleted in November. Choose one now, or keep the intention open?

- A. Keep the platform open, and record the full ordered gate list (fmt → clippy → boundaries →
  test → coverage → frontend test/lint/build/format → e2e → size) so any platform can implement it
- B. Adopt GitHub Actions now and implement that list
- C. Adopt a different platform (say which)
- D. Run the gates manually with no recorded order
- X. Other (please specify)

[Answer]: A — keep the CI platform open, and record the full ordered gate list. **Mode:** guided

## Q6. [operations engineer] Is the self-hosted observability stack still the target?

Our memory says we deploy the Loki/Prometheus/Tempo/Grafana stack, but no such service exists, no
Prometheus scrape configuration exists, and no OTLP exporter is wired — services log to stdout and
expose `/metrics` that nothing scrapes. What should the practice record?

- A. Keep it as the target, and say plainly that structured logs exist today while metrics and
  traces are not collected
- B. Drop the self-hosted stack from our practice
- C. Keep it as the target and make a collector/scrape configuration a deliverable of this intent
- D. Leave the current wording alone
- X. Other (please specify)

[Answer]: A — keep the self-hosted stack as the target, and say plainly that structured logs exist today while metrics and traces are not collected. **Mode:** guided

## Q7. [release engineer] Migration ordering and the automatic down migration

Nine services ship a `*-migrate` one-shot that the release job is supposed to run before the API
and worker update, but no release job exists; `asset-registry` and `audit` have neither a migrate
binary nor a migrate service; and nothing applies a `.down.sql` except one test. This intent adds
Identity migrations. What should the practice commit to?

- A. Record migrate-before-update as the release-job order, and make the automatic down step an
  explicit deliverable of this intent
- B. Record the order now but leave the down step as a separate future deliverable
- C. Leave both open and settle them in delivery planning
- D. Record nothing; keep the existing down-migration rule as it stands
- X. Other (please specify)

[Answer]: A — record migrate-before-update as the release-job order, and make the automatic down step an explicit deliverable of this intent. **Mode:** guided

## Q8. [release engineer] Release cadence and image tags

There are zero git tags and every image reference defaults to mutable `:latest`. What should the
practice say?

- A. Continuous from `main`, images tagged with the commit SHA, no semver ceremony until a first
  production release
- B. Semver tags on each release
- C. Leave cadence and tags open
- D. Keep `:latest` and add no convention
- X. Other (please specify)

[Answer]: A — continuous from `main`, images tagged with the commit SHA, no semver ceremony until a first production release. **Mode:** guided

## Q9. [developer] Correcting the ambient rules file, and the canonical command list

Memory will be corrected on promotion (Yarn 4.18.1 instead of the unsatisfiable "Yarn ≥ 6", and
the observability stack described as a target rather than deployed), but `.claude/rules/
development.md` still says the old thing. Also, `web/` now defines `yarn e2e` and `yarn size`.
What should happen?

- A. Accept the memory corrections now, leave the rules-file edit to its owner as a recorded open
  action, and add `yarn e2e` and `yarn size` to the canonical command list
- B. Correct `.claude/rules/development.md` as part of this stage as well
- C. Only add the two commands; leave the wording alone
- D. Leave both alone
- X. Other (please specify)

[Answer]: A — accept the memory corrections now, leave the rules-file edit to its owner as a recorded open action, and add `yarn e2e` and `yarn size` to the canonical command list. **Mode:** guided

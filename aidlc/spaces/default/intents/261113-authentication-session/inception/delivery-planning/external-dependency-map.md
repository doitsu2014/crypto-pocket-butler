# External Dependency Map — Authentication & Session Management

A **Bolt** is one build pass over one Unit that ends in a running result. Unit boundaries are defined in `unit-of-work.md` and their direct dependency DAG is in `unit-of-work-dependency.md`. This map identifies
external services, infrastructure and human-owned decisions that can block a Bolt, names an owner,
and records a fallback or mitigation. Existing Identity, Gateway, Notification and Audit services are
internal project dependencies, not outside vendors or teams.

| Dependency | Owner | Blocks | Needed by | Fallback / mitigation |
|---|---|---|---|---|
| Supabase Auth behavior and Admin API/JWKS configuration | Supabase platform operator / Security | U1 policy adapter; U2 provider adapter; U3 callback/enrolment; U4 provider-failure proof | Working local/test provider configuration before U1 integration; pinned provider API behavior before U2 handlers | Mock provider for unit/browser tests; fail closed on missing/stale JWKS; already-issued tokens continue only to their expiry during outage |
| Supabase Auth service credential in Vault-backed Docker secret | Security / Operations | U2 privileged revoke/factor operations; U4 integration proof | Before provider Admin API integration tests | Never commit or put in `.env`; test fixture secret only in isolated local test setup; fail clearly if missing |
| Disposable PostgreSQL with pgsodium | Quality / Operations | U1 repository/migration tests; U2 adapter tests; U4 full gate/coverage | Before every blocking ignored-suite and coverage run | Use pinned `supabase/postgres:15.8.1.060`, superuser, isolated throwaway database; no shared or production DB. No committed provision script exists, so the gate is blocked until a safe local invocation is available |
| Notification service availability and internal route | Notification service owner (internal) | U1 alert port behavior; U2 outbound adapter; U4 retry/idempotency proof | Contract, service token and route available before U2 integration | Sign-in must succeed if notification fails; bounded retry with stable idempotency key; persist/report the failure instead of silently dropping it |
| Supabase Auth email delivery (magic links and verification) | Supabase configuration / Operations | U3 magic-link and verification journeys; U4 browser tests | Configured SMTP/Mailpit for local end-to-end validation | Mock delivery in browser tests; use local Mailpit when available; no email-provider secret in the SPA |
| Recovery approver policy and admin-session acceptance | Product / Compliance / Security | U1 recovery authorization; U2 support/admin endpoints; U4 authorization proof | Must decide whether recovery reuses the distinct-admin support-grant approver and define authorized-admin session operations before the affected handler is frozen | Block only those operations; do not guess. Other U1/U2 work can proceed where independent, but dependent stories cannot be marked complete |
| Session absolute-cap anchor | Security | U1 token/session policy; U2 claims/refresh contract; U3 expiry copy; U4 expiry tests | Requirements Q3 is still open; decide whether cap is from initial sign-in or last refresh before affected design/implementation | Leave the policy unset in implementation instructions; do not assume a default |
| Verification resend authentication mode | Product / Security | U2 resend route; U3 verification screen; U4 non-enumeration test | Resolve authenticated-user versus neutral public action consistently across contract, gateway and handler | Do not implement until the contract's auth class is coherent |
| Unusual-location definition | Security / Product | U1 new-device alert eligibility; U4 alert tests | Define a measurable trigger before implementing the location branch | Deliver only the explicit new-device trigger and mark unusual-location as deferred/open; do not infer geography from an unapproved heuristic |
| Auth-route / admin story reconciliation | Product / Architect | U2 admin session endpoint; U4 route completeness | Resolve FR4.7 story acceptance and route mapping after the review finding | Preserve the requirement, but do not claim its coverage complete until it has an approved criterion |
| Reviewed design findings (CTA contrast; missing step-up/change screens) | Design / Product | U3 user interface and E2E visual/accessibility proof | Must disposition before implementing affected screens | Keep the product-lead findings visible; use a compliant existing token treatment and add missing surfaces only after the required human decision/revision path |

## Internal project dependencies (not external hand-offs)

| Dependency | Consumer Unit | Contract / mitigation |
|---|---|---|
| Identity use cases and ports | U2 | U2 depends on U1; no parallel implementation is planned |
| Declared gateway routes and HTTP contracts | U3 | U3 depends on U2; SPA calls only Gateway |
| Notification and Audit services | U1/U2 | Synchronous internal HTTP / existing outbox, bounded retry, idempotency; no message queue |
| Human reviews and Unit approvals | Every Unit | Human remains the decision owner; a review verdict or test pass does not approve a Unit |

## Dependency handling rules

1. Every dependency uses the exact owner and fallback recorded above; blocked work is named rather
   than represented as a passing test.
2. A failed external call never emits success, silently drops an audit event, exposes upstream detail,
   or triggers an unbounded retry.
3. Only Supabase Auth is directly called from the browser; all application APIs pass through the
   gateway.
4. Secrets are loaded from Vault-backed `*_FILE` configuration; no provider or service credential
   is stored in source or an env file.
5. The project's disposable database and auth browser journeys are separate test dependencies. A
   green unit test alone does not replace either gate when the corresponding boundary is changed.

# Risk and Sequencing Rationale — Users, Roles & Permissions

## Heuristic

**Walking-skeleton first** (Cockburn, *Crystal Clear*): the first three Bolts deliver the thinnest end-to-end path — Jurisdiction Gate → User Registry → Sign-up Gates UI — through the real API Gateway, before the wider permission model is built. The alternative heuristics were considered and rejected for this work:

- **Risk-first** would have started with Authorisation and Support Access. Rejected because those rules are unobservable until a principal exists, and a failed end-to-end wiring discovery late would invalidate them.
- **Value-first** would have started with owner settings. Rejected because settings depend on the registry and authorisation anyway, and it would prove less per Bolt.
- **WSJF scoring** was not used: with ten similarly sized Units and a small team, a scoring model adds ceremony without changing the order the DAG and the skeleton already imply.

## Why this order

The order is the dependency graph with the skeleton slice pulled to the front:

1. **B1 Jurisdiction Gate** has no dependencies; it also enforces the "nothing stored" promise the registry relies on.
2. **B2 User Registry** is the first real principal, so it can be proven immediately.
3. **B3 Sign-up Gates UI** closes the slice through the gateway — the first point where a user-facing journey is real.
4. **B4 Authorisation** follows the principal, because every later unit calls it.
5. **B5 Preferences** and **B6 Lifecycle** are independent of each other and of Support Access; they go next.
6. **B7 Support Access** needs both the registry and authorisation.
7. **B8–B10 UI** follow the services they call.

No deviation from topological order is required.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The Supabase Auth Admin API write path for `app_metadata.role` is unproven | Medium | High | Prove it in B4 with a disposable local Supabase; keep the Vault-held service credential; fail the Bolt if the write cannot be audited with previous and new value |
| The support metadata projection has no data source outside the identity schema | High | High | Resolve in B7: read Account and Portfolio over their HTTP APIs, or agree an aggregation path; do not cross schemas |
| The four-status model collides with the stored `pending` status | Medium | Medium | Resolve the migration in B2/B6 before either Bolt closes; one status set, one migration pair |
| The status/role change takes effect only after token refresh | Medium | Medium | State the behaviour in the contract and the UI copy; do not promise immediate revocation |
| Fresh users are denied by the gate if geolocation is unavailable | Medium | Medium | Fail closed (nothing stored) and surface a retry; never fail open |
| The jurisdiction list is compliance-owned and not yet populated | High | Medium | Seed the launch entries before B1 closes; record the list owner in the external dependency map |
| The walking skeleton's first unit is a prerequisite, not a shipped slice | Certain | Low | The skeleton checkpoint is reached at the end of B3, verified by the intent's check |
| Frontend UI units reuse components across two unlinked Yarn packages | Medium | Low | Each package defines its own copy of RoleBadge, StatusPill, and GrantBanner, or a shared package is introduced with explicit dependency edges |

## Confidence hypotheses

Each Bolt is judged by what it proves, not by what it finishes:

- B1 → a restricted attempt stores nothing.
- B2 → provisioning is idempotent and stable across provider changes.
- B3 → the full gate journey works through the real gateway.
- B4 → one decision point for role, status, and ownership, with an auditable role write.
- B5 → preferences and consent survive and are owned.
- B6 → the status machine cannot be bypassed; erasure is repeatable.
- B7 → support can help with metadata and never reach portfolio contents.
- B8–B10 → each surface completes its actor's journey without a credential field or an unavailable state.

# Bolt Plan — Users, Roles & Permissions

A **Bolt** is one build pass over a piece of the work, ending in something that runs. This plan puts one Unit of Work in each Bolt and orders them walking-skeleton-first: prove a thin end-to-end path before widening. The order respects the dependency graph from Units Generation.

**Upstream:** `requirements.md` (FR14–FR20), `unit-of-work.md`, `unit-of-work-dependency.md`, `unit-of-work-story-map.md`, `contract-summary.md`, `stories.md`, `mockups.md`, `team-practices.md`.

## Sequence

| Bolt | Unit | Kind | Walking skeleton? | Definition of Done | Confidence hypothesis | Expected demo |
|---|---|---|---|---|---|---|
| B1 | `u6-jurisdiction-gate` | service | Prerequisite of the slice | The gate evaluates a self-declared country plus an IP signal against the maintained list, stores nothing on a block, and its tests pass (hot path + blocked + geolocation-unavailable) | A restricted attempt can be refused before any personal data exists | A request for a restricted country returns a refusal with no user row written |
| B2 | `u1-user-registry` | service | Part of the slice | One record per provider subject, stable id, profile fields, status field, subject hash and shredding-key reference; provisioning refuses when the gate has not cleared; `/v1/principals` route declared | Provisioning is idempotent and safe across provider claim changes | First sign-in creates exactly one record; a second sign-in reuses it |
| B3 | `u8-signup-gates-ui` | ui | **Completes the slice** | Terms, risk disclosure, age, and country are collected; a missing acceptance is named; a blocked country shows the notice; the screen reads the gate and the registry through the gateway only | A visitor can pass the gates end to end through the real gateway | A blocked sign-up stores nothing; an allowed one reaches the first-source step |
| B4 | `u2-authorisation` | service | — | Role mapping, deny-by-default, status denial, ownership contract, denial evidence; the role write path through the Supabase Auth Admin API | Every decision is one place, and a role change is auditable with previous and new value | A non-admin is denied; an admin is allowed; a role change is recorded |
| B5 | `u4-preferences-consent` | service | — | Preferences and consent history with ownership checks; provider defaults never overwrite a user's choice | Preferences and consent are durable and auditable | A locale change persists; a consent withdrawal is recorded |
| B6 | `u5-account-lifecycle` | service | — | The four-status machine, admin disable/restore, deletion request with the 30-day recovery, legal-hold-aware erasure, idempotent re-run | The status machine cannot be bypassed and erasure is safe to repeat | Delete, recover within 30 days, and erase on day 31 |
| B7 | `u3-support-access` | service | — | Grant request, different-admin approval, 60-minute maximum, the metadata projection boundary, expiry and revocation, every access audited | A support agent can help with metadata and can never reach portfolio contents or credentials | A grant is requested, approved by another admin, used, and expires |
| B8 | `u7-settings-ui` | ui | — | Profile, preferences, consent, deletion and recovery surfaces; blocked-account screen; role badge and status pill | The owner can run their whole account journey without support | Settings save; deletion starts the recovery banner; restore works |
| B9 | `u9-support-console-ui` | ui | — | Grant request and approval, the grant banner, the metadata panel, least-privilege presentation | Support can complete the metadata journey without a credential field in sight | Request → approve → read metadata → expire |
| B10 | `u10-admin-users-ui` | ui | — | User search, role and status change with reason, grant approval, the same metadata boundary | Admin work is correct, reasoned, and audited | Look a user up, change a role with a reason, see the audit note |

## Walking skeleton

The first three Bolts are the thin end-to-end slice: **Jurisdiction Gate → User Registry → Sign-up Gates UI**, reached through the real API Gateway. The team's standing slice (SPA → gateway → Asset Registry) is already built and proven; this slice proves the BRD-002 path joins it.

- **Bolt 1 is a prerequisite, not the shipped skeleton.** The engine completes Bolt 1's full unit slice first; the skeleton checkpoint is reached when Bolt 3's UI journey passes through the gateway.
- **Real integrated check:** a blocked country stores nothing; an allowed visitor passes terms, age, and country and reaches the first-source step — verified by the intent's verification command, not by a design review.
- **Prerequisites:** the gateway route table gains the `/v1` routes for the gate and the principal; the jurisdiction list has at least the launch entries.

## Notes

- The order is DAG-respecting; no deviation is needed.
- Every Bolt lands its tests in the same change (TDD), including one integration test per adapter against a disposable Postgres schema.
- Bolt size was chosen as one Unit per Bolt; each Bolt is independently reviewable and leaves the tree buildable.

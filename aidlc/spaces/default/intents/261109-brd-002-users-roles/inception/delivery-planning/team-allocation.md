# Team Allocation — Users, Roles & Permissions

This file maps each **Bolt** (one build pass over a Unit of Work, ending in something that runs) to its owner. Team Formation was skipped for the classic scope, so there is no human team roster to reference.

## Allocation

All Bolts are executed by **aidlc-developer-agent** (AI), following the team's four-crate Clean Architecture and TDD practices. Reviews are human before merge, as the organisation rule requires.

| Bolt | Unit | Owner | Review expectation |
|---|---|---|---|
| B1 | `u6-jurisdiction-gate` | aidlc-developer-agent | Human review: security rules and audit of blocked attempts |
| B2 | `u1-user-registry` | aidlc-developer-agent | Human review: subject hashing, ciphertext, provisioning idempotency |
| B3 | `u8-signup-gates-ui` | aidlc-developer-agent | Human review: WCAG, missing-acceptance copy, nothing stored on block |
| B4 | `u2-authorisation` | aidlc-developer-agent | Human review: deny-by-default, role-write path and audit |
| B5 | `u4-preferences-consent` | aidlc-developer-agent | Human review: ownership checks, consent history |
| B6 | `u5-account-lifecycle` | aidlc-developer-agent | Human review: status transitions, 30-day window, legal-hold override |
| B7 | `u3-support-access` | aidlc-developer-agent | Human review: requester/approver distinctness, metadata boundary |
| B8 | `u7-settings-ui` | aidlc-developer-agent | Human review: blocked-account screen, destructive-flow confirmation |
| B9 | `u9-support-console-ui` | aidlc-developer-agent | Human review: no credential fields, grant banner accessibility |
| B10 | `u10-admin-users-ui` | aidlc-developer-agent | Human review: reason-required changes, self-approval disabled |

## Working agreements

- One Bolt at a time (serial execution, as recorded).
- Each Bolt is one Unit of Work: domain tests, then application use cases against port fakes, then adapters against a disposable Postgres schema, then the HTTP boundary — or, for a UI Bolt, component contract, screens, then MSW-backed tests.
- Every pull request links to its requirement or unit id, per team practice.
- Review looks at correctness and security before style.

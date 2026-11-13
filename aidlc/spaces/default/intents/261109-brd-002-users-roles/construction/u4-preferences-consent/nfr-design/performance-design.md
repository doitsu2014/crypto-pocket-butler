# Performance Design — u4-preferences-consent

**Upstream:** `performance-requirements.md` (NFR2.1–NFR2.7), `functional-spec.md` (W0–W5), `tech-stack-decisions.md`.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| State query | One `SELECT DISTINCT ON (kind) … ORDER BY kind, occurred_at DESC, seq DESC` | Four rows, one index scan, no history read |
| Preference read before first write | No query: defaults computed in the service | The common case is a user who has never opened settings |
| Preference write | `INSERT … ON CONFLICT (user_id) DO UPDATE SET` only the supplied columns | One statement, no read-modify-write race |
| Withdrawal | One row-locked read for the version, one insert, one outbox insert — all in one transaction | Serialisation and durability in one commit, with no network call inside it |
| Audit delivery | Deferred to the outbox drainer | The C9 hop never sits in a user-facing latency budget (NFR1.5) |
| History read | Indexed `(user_id, occurred_at DESC, seq DESC, id)` with `LIMIT 100` | Paged, never a full-history scan |

## Budgets as designed

| Path | Budget |
|---|---|
| Preferences read, no row | p95 ≤ 20 ms |
| Preferences read, row present | p95 ≤ 40 ms |
| Preferences write | p95 ≤ 50 ms |
| Consent-state query | p95 ≤ 40 ms |
| Consent grant | p95 ≤ 50 ms |
| Consent withdrawal | p95 ≤ 80 ms (lock + read + insert + outbox) |
| Consent history, newest 100 | p95 ≤ 60 ms |

## What is deliberately absent

No cache: the gate must see a withdrawal immediately (NFR24.1), and a four-row read does not
need one. No `SELECT *` history read. No network call on the request path. No batch endpoint —
the caller asks for one user at a time.

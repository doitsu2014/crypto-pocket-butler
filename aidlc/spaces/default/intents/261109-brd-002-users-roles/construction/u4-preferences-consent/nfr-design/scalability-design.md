# Scalability Design — u4-preferences-consent

**Upstream:** `scalability-requirements.md` (NFR20.1–NFR20.6), `performance-design.md`, `tech-stack-decisions.md`.

## Scaling architecture

- **Stateless.** No cache, no session, no cross-replica state (NFR20.1).
- **Row growth, not cost growth.** Every read is by `user_id` on an index, so one user's state costs
  the same at any table size (NFR20.2).
- **Consent events** grow at 4–20 rows per user; the table is append-only until erasure.

## Indexes

| Index | Purpose |
|---|---|
| `user_preferences (user_id)` primary key | One row per user |
| `consent_records (user_id, kind, occurred_at DESC, seq DESC)` | The state query: newest per kind |
| `consent_records (user_id, occurred_at DESC, seq DESC, id)` | The history page |
| `consent_audit_outbox (next_attempt_at) WHERE state = 'pending'` | The drainer's due-row selection: `WHERE state = 'pending' AND next_attempt_at <= now()` |
| `consent_audit_outbox (delivered_at) WHERE state = 'delivered'` | The 24-hour cleanup delete; without a delivered-row index the cleanup would scan the outbox |

## Retention and erasure

- Consent rows live until the account is erased (NFR20.5); the erasure job deletes them through
  `identity_retention`. There is no age-based pruning, because a consent record is evidence.
- Until BRD-017's legal-hold register exists, no row is exempt (NFR20.6); when it lands, the
  erasure job's own check keeps a held row and records that on its audit entry.
- Delivered outbox rows are deleted after 24 hours so a failed delivery can still be replayed within
  that window (NFR1.6).

## Capacity thresholds

| Threshold | Value | Action |
|---|---|---|
| Consent rows | 100 M | Review partitioning; not expected soon at 4–20 rows/user |
| State-query p95 | > 40 ms for 10 minutes | Check the index before adding replicas |
| Outbox backlog | any row undelivered > 15 minutes | Page; the drainer or C9 is failing |
| CPU per replica | 70% sustained | Add a replica |

## Concurrency

- **Withdrawals serialise per user** on the newest event row (`FOR UPDATE`), so two racing
  withdrawals cannot both read `granted`; the second sees `withdrawn` and returns the no-op.
- A grant and a withdrawal racing are serialised by the same lock; `seq` orders them.
- Preference writes are per-field last-writer-wins; no lock, because each write carries only the
  fields its caller supplied.
- The drainer takes outbox rows in `FOR UPDATE SKIP LOCKED` batches, so several replicas can drain
  without double-posting; the idempotency key makes a rare double-post harmless anyway.

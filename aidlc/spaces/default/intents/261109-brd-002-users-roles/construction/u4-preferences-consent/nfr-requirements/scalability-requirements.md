# Scalability Requirements — u4-preferences-consent

**Upstream:** `functional-spec.md`, `performance-requirements.md`, `requirements.md` (FR17.3, NFR21, NFR24).

## Load projection

| Dimension | Target | Basis |
|---|---|---|
| Preferences rows | one per user | One row per account, keyed by user id |
| Consent events | roughly 4–20 per user over an account's life | Grants and withdrawals across four kinds; append-only, so growth is linear in user actions |
| Consent-state query | one per source-connection attempt plus one per settings screen | The busiest path — once per connection, not once per kind |
| Outbox rows | one per withdrawal | Drained and marked delivered; the table stays small |
| Identity API replicas | 3 (as configured) | Existing stack |

## Scaling behaviour

- **Stateless.** No cache, no session, no cross-replica state (NFR20.1).
- **Growth is row count, not cost per request.** Every read is indexed by user id, so one user's state costs the same number of row reads at ten million users as at ten (NFR20.2).
- **No pruning inside this unit.** Consent events live until the account is erased (below). The outbox is drained, not pruned: a delivered row is deleted by the drainer after a short retention window so a failed delivery can be replayed (NFR20.3).

## Retention and erasure

| ID | Requirement |
|---|---|
| NFR20.5 | Consent events and preferences are retained **until the account is erased**, not "for the life of the account" in the sense of forever: the erasure job deletes them through `identity_retention` (NFR22.3). This is the one path by which these rows leave the database, and it is required by FR17.3. |
| NFR20.6 | Until BRD-017's legal-hold register exists, no consent row is exempt from erasure. When the register lands, a held account's rows are kept by the erasure job's own check, and that exception is recorded on the erasure's audit entry — not by widening the application role's privileges. |

## Capacity thresholds

| Threshold | Value | Action |
|---|---|---|
| Table size | 100 M consent rows | Review indexing and partitioning; not expected in the first years at 4–20 rows per user |
| State-query p95 | above 40 ms for 10 minutes | Investigate the index before adding replicas |
| Outbox undelivered rows | any row older than 15 minutes | Alert; the drainer or C9 is failing |
| Sustained CPU per replica | 70% | Add a replica |

## Concurrency

- **Withdrawals are serialised per user.** The withdrawal transaction takes a row lock on the user's newest event (`SELECT … FOR UPDATE`) before deciding, so two racing withdrawals cannot both read `granted` and both insert: the second waits, sees `withdrawn`, and returns the no-op. The claim is backed by the lock, not by timing.
- A grant and a withdrawal racing are serialised by the same lock; the sequence orders them deterministically and the history shows both.
- Preference writes are whole-field; two clients writing the same field are last-writer-wins, and every write validates its own supplied values.

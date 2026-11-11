# Scalability Requirements — u5-account-lifecycle

**Upstream:** `functional-spec.md`, `performance-requirements.md`, `requirements.md` (NFR24).

## Load projection

| Dimension | Target | Basis |
|---|---|---|
| Status transitions | 2–8 per account lifetime | Disable/restore, request/restore, expiry |
| `deletion_requests` rows | at most one open per user; cancelled and erased rows accumulate | Evidence, retained with the account's record |
| `status_transitions` rows | one per transition | Append-only |
| Expiry sweep | one pass per 60 s over due requests | The existing `IDENTITY_SWEEP_INTERVAL_SECONDS` |
| Identity API replicas | 3 (as configured) | Existing stack |

## Scaling behaviour

- **Stateless request paths.** The three user-facing operations are single transactions (NFR20.1).
- **Bounded batches.** The sweep claims at most 100 requests per pass with the existing lease; a
  backlog lengthens the number of passes, never a single pass (NFR20.2).
- **Growth is row count.** Every read is indexed by `user_id` or by `(status = open, recoverable_until)`
  for the sweep; cost per request is constant in table size (NFR20.3).

## Capacity thresholds

| Threshold | Value | Action |
|---|---|---|
| Due-request backlog | > 500 open requests past their deadline | Alert; the sweep is not keeping up |
| Sweep batch duration | > 60 s | Reduce the batch size or add a replica |
| `status_transitions` size | 50 M rows | Review partitioning; not expected at 2–8 per account |

## Concurrency

- The lease (`lease_owner`, `lease_until`) makes a request claimable by one worker at a time; a
  crashed claimer's lease expires and the row returns to the pool.
- A user's restore and the sweep race: the sweep re-reads the status inside the lease and closes a
  restored request instead of erasing it (NFR24.2).
- Two admin operations on one account are last-writer-wins on the status, and both transitions are
  recorded; the machine's edge check refuses the second if the first moved the status out of its
  `from` value.

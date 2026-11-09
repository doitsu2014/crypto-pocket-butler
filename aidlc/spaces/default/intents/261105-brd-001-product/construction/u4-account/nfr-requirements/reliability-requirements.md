# Reliability Requirements — u4-account

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR1.1 | C5 read success | 99.9% over 30 days; rolling update health-gated | NFR1 |
| NFR18.1 | RPO ≤24 h, RTO ≤4 h for the Account schema | Supabase backup regime | NFR18 |
| NFR4.2 | A transient provider failure is retried within bounds and applied once | 3 attempts with jitter; idempotency | NFR4, FR10.3 |
| NFR8.1 | A failed sync keeps and labels previous holdings | never zeros or fabricates | NFR8, FR2.5 |
| NFR1.2 | A killed sync blocks no later sync beyond one lease | 60-second run lease, 60-second sweep; the run is marked failed and the guard released | FR2.5, NFR1 |

## Fault tolerance

- **Venue/RPC unavailable**: the run fails with a category, keeps the last snapshot, and notifies;
  the next schedule retries.
- **Rate limit**: back off and report `rate_limit`; no data is lost.
- **Registry unavailable**: the run fails; holdings keep the last snapshot.
- **Audit unavailable**: the event is retried and surfaced; security-sensitive events are not
  silently dropped.
- **Worker dies mid-run**: the lease expires, the run is marked failed, and the guard is released.
- **Import interrupted**: the batch is idempotent by `dedupeKey`; a retry skips stored rows.

## Graceful degradation

- C5 reads serve the last snapshot when a sync is failing.
- A spam holding is returned, never dropped.
- The account list always shows status and last-synced time.

## Backup

- Supabase backup regime covers the Account schema; a restore rehearsal runs before launch.

NFR references: NFR1.1, NFR1.2, NFR4.2, NFR8.1, NFR18.1.

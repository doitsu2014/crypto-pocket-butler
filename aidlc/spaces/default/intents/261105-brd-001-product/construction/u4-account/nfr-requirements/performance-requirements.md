# Performance Requirements — u4-account

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

Account is on the portfolio read path through C5 and runs provider syncs in the background.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR2.1 | `getHoldings` at p95 | ≤ 150 ms | NFR2 |
| NFR2.2 | Account list at p95 | ≤ 100 ms | NFR2 |
| NFR2.3 | Manual re-sync acknowledgement at p95 | ≤ 200 ms (202 with the run id) | NFR2 |
| NFR4.1 | Single-account sync end-to-end | ≤ 30 s typical | NFR4 |
| NFR6.1 | First sync after adding a wallet / exchange account | starts immediately; the first sync completes and holdings are available within 2 min (wallet) / 10 min (exchange); the end-to-end valuation is verified downstream by u5-market-data and u6-portfolio | NFR6 |

## Unit-level target (no inception NFR behind it)

- Import of a 10k-row CSV: ≤ 60 s, streaming, per-line errors. This is a unit-level design target
  with no inception NFR; it does not claim an NFR ID.

## Latency budget and per-dependency timeouts

- Provider calls: venue/RPC connect 1 s, read 5 s, total 30 s; the sync path retries within the
  bound.
- The C5 holdings read is one indexed query per account plus the snapshot rows; no provider call on
  the read path.

| Dependency | Connect | Read | Total | Retries |
|---|---|---|---|---|
| C3 grant check (support read) | 1 s | 2 s | 5 s | 2, inside the NFR2.2 budget |
| C4 resolve / chains | 1 s | 3 s | 10 s | 3 |
| C9 alert | 1 s | 3 s | 10 s | 3 with an idempotency key |
| C10 audit | 1 s | 2 s | 5 s | one attempt for security-critical events; surfaced on failure |
| C15 Auth admin claim update | 1 s | 3 s | 10 s | 3 with an idempotency key |
| Binance / OKX / RPC | 1 s | 5 s | 30 s | 3 with backoff |

The support metadata read's C3 call is inside the NFR2.2 100 ms p95 budget only when the grant is
already cached for the request; otherwise the read returns 202 with a check pending and the caller
retries.

## Throughput

- One sync per account at a time; a batch of accounts is worked serially by a worker or in the
  scheduler's bounded concurrency.
- The first sync starts on add; the schedule runs at the cadence Job Orchestration sets.

## Localization

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR16.1 | Labels and status text cross the boundary as stored | No formatting in the service | NFR16 |

NFR references: NFR2.1, NFR2.2, NFR2.3, NFR4.1, NFR6.1, NFR16.1.

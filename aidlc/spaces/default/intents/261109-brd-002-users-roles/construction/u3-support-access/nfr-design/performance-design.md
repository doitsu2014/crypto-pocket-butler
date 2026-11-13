# Performance Design — u3-support-access

**Upstream:** `performance-requirements.md` (NFR2.1–NFR2.11), `functional-spec.md` (W1–W5),
`contract-summary.md` (C10 timeout and retry rules), `tech-stack-decisions.md`.

## Where the time goes

| Path | Fixed work | External hops | Budget (p95 / p99) |
|---|---|---|---|
| Grant request (accepted) | JWT already validated; role from claims; one target-user read; one insert of the grant plus its outbox row in one transaction | none | 80 / 180 ms |
| Grant request (refused) | Role check fails before any read, or the live-grant check answers; one outbox insert in its own transaction | none | 60 / 150 ms |
| Approve / revoke (accepted) | one grant read, one conditional `UPDATE` plus its outbox row | none | 80 / 180 ms |
| Approve / revoke (refused) | the conditional update matches zero rows, or the stale/terminal branch | none | 60 / 150 ms |
| Grant evaluation | one indexed read on `(user_id, status, expires_at DESC)`, clock comparison in the domain | none | 15 / 40 ms |
| One metadata source | one internal GET | Account or Portfolio | 60 / 80 ms |
| Metadata read (served) | evaluation, concurrent fan-out, one synchronous audit append | Account, Portfolio, audit | 150 / 300 ms |
| Metadata read (refused) | evaluation only; no hop is made | none | 60 / 150 ms |
| Metadata read (any branch) | the 500 ms deadline starts before the grant read and encloses pool acquire, both one-statement transactions, the hops and the append | enforced | ≤ 500 ms wall clock |
| Support outbox pass | claim, then up to 5 sequential deliveries | audit | ≤ 10 s worst case; typically < 500 ms |

## The read pipeline

```
┌─ tokio::time::timeout(500 ms) ── the whole request, from before the first statement ─┐
request ──▶ authenticate (middleware) ──▶ load grant (one statement)
        ──▶ domain decides: exists? holder? status valid at now? target not pending_deletion?
              ├─ no  → refusal outbox row (own tx) → 403/409/422 or 503 audit-unavailable
              └─ yes → fan-out: tokio::try_join!(account.get(user), portfolio.get(user))
                        each with a 120 ms request timeout
                        ├─ either fails/times out → 503 metadata-source-unavailable (+ audit `failed`)
                        └─ both answer → synchronous audit append (200 ms, one attempt)
                                          ├─ append fails → 503 audit-unavailable
                                          └─ append ok → 200 projection
└──────────────────────────────────────────────────────────────────────────────────────┘
```

Two properties make the numbers hold:

1. **A refusal never fans out.** Grant evaluation answers before either source is called, so the
   common refusal costs one indexed read and one outbox insert — 60/150 ms, not 300.
2. **The fan-out is concurrent, not sequential.** `tokio::try_join!` means the read costs the
   slower source (p99 80 ms), not their sum. A sequential design would add ~80 ms and break the
   300 ms p99.
3. **The 500 ms deadline encloses everything, including the steps before the fan-out.** It is
   started before the grant read — not after evaluation — so it also bounds the pool acquire (a 5 s
   acquire setting would otherwise outlast it), both 500 ms statement bounds, the two hops and the
   append. Whichever bound is reached first cancels the request; a cancelled one-statement
   transaction rolls back. The wall clock a caller observes is therefore 500 ms on every branch,
   which is what NFR2.10 asks for.

## Concurrency and connections

- The two source calls run on the service's existing `reqwest` client, one per source, each built
  with `connect_timeout(1s)` and a per-request `.timeout(120ms)`. The request's overall deadline is
  `tokio::time::timeout(500ms, …)` started **before the grant read**, around the entire pipeline:
  pool acquire, the grant and target-user statements, the fan-out and the append. A timed-out
  request is cancelled, not abandoned in the background, and the 500 ms wall clock is the same on
  the served, refused and unavailable branches.
- **No database connection is held across the fan-out.** The grant read and the target-user read
  each run in a one-statement transaction (`BEGIN; SET LOCAL statement_timeout = '500ms'; …;
  COMMIT;`) on a pooled connection that is released before the first source call. The 500 ms
  statement bound is therefore real; `SET LOCAL` outside a transaction would silently do nothing.
  Because the overall deadline began before these statements, a saturated pool cannot stretch the
  request past 500 ms — the acquire is cancelled with the rest of the pipeline.
- Mutations set `SET LOCAL statement_timeout = '2s'` and hold their connection for one short
  transaction (grant write + outbox insert), never across a network call.
- The pool is the service's existing one — acquire 5 s, idle 600 s, max lifetime 1800 s. The unit
  adds no second pool and no dedicated bulkhead: support traffic is staff traffic and cannot
  exhaust a pool sized for the product's own request rate.

## Outbox pass

The support sink is added to the existing drainer's `run_once`, not to a new loop. It has its own
batch (5), its own client (1 s connect, 2 s request, one attempt per row), and the shipped claim
lease (30 s). 5 × 2 s = 10 s worst case, so a claimed row is never re-claimable while its pass is
in flight. The consent and status sinks keep their shipped settings; the support sink changes
nothing they use.

## What is deliberately absent

- **No cache.** The projection is assembled per request (BR3.7, NFR24.1); there is no cache to
  size, warm, or invalidate, and no stale projection to serve under a revoked grant.
- **No pagination or lazy loading.** The projection is one user, and each list it carries is
  bounded by the product's own limits (accounts and portfolios per user), not by support traffic.
- **No pre-computation or materialised view.** A projection table would copy another service's
  data, which the no-cross-schema rule forbids; `tech-stack-decisions.md` records the rejection.

## Measurement

The read-duration histogram (`support_read_duration_seconds`, labelled by `outcome`) is bucketed at
25, 50, 100, 150, 200, 300, 500, 1000 ms so the 150/300 ms targets are readable from it; the
write paths use the existing HTTP-layer duration metric with the route label. The budgets are
guarded by those metrics and their alerts, not by a benchmark harness (NFR2.11's note).
# Performance Requirements — u6-jurisdiction-gate

**Upstream:** `functional-spec.md` (W1–W3), `rules.md` (BR1.1, BR1.5, BR1.6), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR2.1 | Gate-only latency | p95 ≤ 100 ms server-side, excluding the provider and the audit write | BRD-001 NFR2; BR1.1 |
| NFR2.2 | End-to-end latency | p95 ≤ 1.5 s including the provider retry and the audit write | BRD-001 NFR2; C8 |
| NFR2.3 | Provider budget | 800 ms per attempt, one retry, 1.5 s total; when the budget is exhausted the check returns `503` and fails closed | BR1.5; C8 `503` |
| NFR2.4 | Audit-write budget | 300 ms per write, one retry, idempotency key; a failed write is surfaced, never swallowed | BR1.7; C9 |
| NFR2.5 | List lookup | Served from memory after load; at most one database read per list version | BR1.6 |
| NFR2.6 | List publish | A new version validates and activates in ≤ 5 s for up to 300 entries | W3 |

The provider and audit budgets together fit under the end-to-end target: 800 ms attempt + 800 ms retry is capped at 1.5 s, and the audit write is allowed 300 ms beside it; the sum is bounded by the gate's own 1.5 s cap, which returns `503` rather than overrunning.

# Performance Design — u6-jurisdiction-gate

**Upstream:** `performance-requirements.md` (NFR2.1–NFR2.6), `functional-spec.md`.

## Latency budget

| Stage | Budget | Design |
|---|---|---|
| Request parse and validation | 5 ms | In-process validation (BR1.4) |
| List lookup | < 1 ms | Active version held in memory; refreshed every 10 s (NFR20.5) |
| Provider call | 800 ms attempt, one retry, 1.5 s cap | Bounded HTTP client; a retry runs only on timeout/5xx; the total cap returns `503` |
| Decision assembly | < 1 ms | Pure function over the two signals |
| Audit write | 300 ms, one retry, idempotency key | Asynchronous to the response? No — the decision completes only when the write is accepted; the write runs inside the budget |
| **End to end** | **≤ 1.5 s p95** | The gate's own total cap |

## Optimisations

- **Memory-resident list** — the active `JurisdictionListVersion` is loaded once per refresh interval; no per-request query (NFR2.5).
- **Decision cache** — keyed by (country code, salted IP hash), TTL 5 minutes, holds no raw IP; serves repeat checks inside the re-evaluation window without a provider call (NFR20.3).
- **Provider connection reuse** — a pooled HTTP client with a 5 s connect timeout and the 800 ms read timeout.
- **Bounded payloads** — the check request and response are small fixed-shape JSON documents.

## Resource pooling

- One database pool for list reads and audit writes, sized per replica (min 2, max 10).
- One HTTP client pool for the provider, capped to the provider quota (600 calls/min per replica).

## Performance tests

- A load test at 200 checks/s measuring the histogram against the p95 target.
- A provider-slowness test proving the 1.5 s cap returns `503` and never an allow.
- An audit-slowness test proving the 300 ms + retry path fails closed.

# NFR Design — u1-walking-skeleton

These questions settle the concrete patterns behind the NFR targets. Answer each with a letter
after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [Architect] How should the gateway call the registry when it fails?

- A. Timeouts (connect 2 s, read 5 s, total 10 s), up to 3 retries with exponential backoff and jitter only for idempotent reads, and a circuit breaker per dependency
- B. No retries; fail fast with 503
- C. Retry aggressively until success
- X. Other (please specify)

[Answer]: A — Timeouts + retries + breaker

## Q2. [Architect] How should the asset cache stay fresh?

- A. Write-side invalidation via a registry change version that replicas poll every 10 s, with a 60 s TTL as a backstop; maximum observable staleness 10 s
- B. TTL only, 60 s
- C. No cache
- X. Other (please specify)

[Answer]: A — Poll + TTL backstop

## Q3. [Architect] How deep should tracing go?

- A. One trace per request spanning gateway and registry with W3C trace context, sampled at 100% at launch because volume is low, with tail sampling introduced later
- B. 10% head sampling
- C. Logs only, no traces
- X. Other (please specify)

[Answer]: A — Full tracing

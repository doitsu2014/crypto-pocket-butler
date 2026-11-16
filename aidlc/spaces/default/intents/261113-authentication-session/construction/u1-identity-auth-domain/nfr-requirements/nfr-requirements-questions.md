# NFR Requirements — U1 Identity Auth Domain Questions

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [quality engineer] Which performance targets should U1 inherit for session reads?

Requirements set the session-list target at p95 ≤ 300 ms and p99 ≤ 800 ms at 50 requests/second per replica. U1 owns the Identity-side session read and policy work; this choice makes its measurable budget explicit.

- A. Apply the same p95 ≤ 300 ms / p99 ≤ 800 ms at 50 requests/second per replica to the Identity session-list operation
- B. Keep the end-to-end target, but allocate U1 p95 ≤ 200 ms / p99 ≤ 500 ms, leaving the remaining budget for gateway and network
- C. Keep the end-to-end target, but allocate U1 p95 ≤ 250 ms / p99 ≤ 600 ms
- D. Defer a U1-specific budget; verify only the end-to-end target
- X. Other (please specify)

[Answer]: A. Apply the same p95 ≤ 300 ms / p99 ≤ 800 ms at 50 requests/second per replica to the Identity session-list operation. **Mode:** guided

## Q2. [operations engineer] What availability/error budget applies to Identity auth operations?

Requirements specify 99.9% monthly availability over 30 days and no more than 0.1% auth-request failures outside a provider outage. The targets need a measurement boundary for this Unit.

- A. Measure successful Identity auth operations at the Identity API boundary; 99.9% monthly availability, excluding planned maintenance only if separately reported
- B. Measure the end-to-end SPA-to-Identity journey against 99.9% monthly availability; Identity retains the same target as a component objective
- C. Apply 99.9% only to the whole platform and treat Identity as an unbudgeted dependency
- D. Leave the availability boundary unresolved; carry only the stated platform target
- X. Other (please specify)

[Answer]: A. Measure successful Identity auth operations at the Identity API boundary; 99.9% monthly availability, excluding planned maintenance only if separately reported. **Mode:** guided

## Q3. [security engineer] How should auth error-rate alerts distinguish provider outages?

Requirements allow provider outages as an exception to the ≤0.1% auth-request failure target, while requiring non-revealing errors and auth-only degradation. This sets alert behavior, not a new authentication fallback.

- A. Alert when non-provider auth failures exceed 0.1% over a rolling 30-minute window; track provider-outage failures separately and alert on their availability impact
- B. Alert when all auth failures exceed 0.1% over a rolling 30-minute window, including provider outages
- C. Alert only on the 99.9% monthly availability burn; do not set a separate error-rate alert
- D. Defer alert thresholds to operations design; require only separate provider-failure and other-failure metrics here
- X. Other (please specify)

[Answer]: A. Alert when non-provider auth failures exceed 0.1% over a rolling 30-minute window; track provider-outage failures separately and alert on their availability impact. **Mode:** guided

## Q4. [quality engineer] What verification is required for concurrency correctness?

The stories require two tabs/devices to refresh or revoke concurrently without losing or resurrecting a session. The requirement names no numeric concurrency load, so this decides the minimum reproducible verification contract.

- A. Deterministic tests for two concurrent actors and repeated idempotent requests, plus a database-backed integration test for conflicting revocations
- B. Add a load test at 50 concurrent actors in addition to deterministic and database-backed tests
- C. Require a 100-actor load test and a zero-failure threshold
- D. Keep only deterministic unit tests; defer database and load verification
- X. Other (please specify)

[Answer]: A. Deterministic tests for two concurrent actors and repeated idempotent requests, plus a database-backed integration test for conflicting revocations. **Mode:** guided

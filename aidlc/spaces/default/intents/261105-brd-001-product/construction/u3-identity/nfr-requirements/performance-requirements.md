# Performance Requirements — u3-identity

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

Identity sits on the request path for every gateway call (principal resolution), and it runs the
export and deletion workers in the background. Its latency budget is therefore tight on the read
path and unconstrained on the background path.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR2.1 | Principal resolution (`resolvePrincipal`) end-to-end at p95 | ≤ 50 ms | NFR2 |
| NFR2.2 | Consent read (`getConsent`) at p95 | ≤ 50 ms | NFR2 |
| NFR2.3 | Consent write and preference write at p95 | ≤ 150 ms | NFR2 |
| NFR2.4 | Session list and revoke at p95 | ≤ 400 ms (includes one Supabase Auth admin call) | NFR2 |
| NFR6.1 | Identity's contribution to the <2 min (by address) / <10 min (by exchange key) first-value target | Sign-up gate ≤ 100 ms server-side; first principal resolution ≤ 50 ms; no Identity step holds the user | NFR6 |
| NFR13.3 | Export assembly runs off the request path | The request wakes the worker; first progress within 2 s; lease 60 s | NFR13 |
| NFR13.5 | Deletion runs off the request path | Acknowledgement ≤ 200 ms; execution in the worker | NFR13 |
| NFR1.1 | Identity availability contributes to the 99.9% monthly target | No single-instance dependency; rolling update health-checked | NFR1 |

## Latency budget

- Gateway to Identity over internal HTTP: ≤ 100 ms p95, including one retry at most.
- Principal resolution is a single indexed lookup by `authUserId` with no cross-service call on the
  hot path; the MFA facts (`aal` and `app_metadata.has_exchange_credentials`) come from the
  validated JWT, so no Account call is made per request.
- The session list serves Identity's `DeviceSession` view; its 400 ms budget includes one optional
  Supabase Auth reconciliation call and no fan-out.
- The synchronous Audit append on a support-read path is bounded to 100 ms at p95 with a 200 ms
  timeout; when the budget is exhausted the read is refused with 503 and the reason.

## Throughput and background work

- Export and deletion are queued by `ExportRequest` and `DeletionRequest` status, not by a message
  queue. A request wakes the worker immediately; the periodic sweep cadence is 60 seconds, and a
  claimed record holds a 60-second lease. Progress is monotonic.
- Worker outbound calls (C5/C7/C8 export and anonymise, Supabase Auth admin, Storage): connect
  timeout 1 s, read timeout 5 s, total timeout 30 s per call; at most 3 attempts with exponential
  backoff and jitter.
- A single worker processes one export or one deletion step at a time; more instances may run, with
  the status transition acting as the single-flight guard.

## Localization preferences

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR16.1 | Identity stores and returns the user's locale, base currency, and timezone without transformation | Preference read ≤ 50 ms; rendering belongs to the Web App | NFR16 |
| NFR16.2 | Identity applies no locale-specific formatting of its own | Values cross the boundary as stored primitives | NFR16 |

## Resource constraints

- The service is stateless apart from Postgres and Vault; no in-memory session or token cache.
- Identity owns its own JWKS cache (public keys only, refreshed when a `kid` it does not hold
  arrives); it caches no tokens or principals across requests.

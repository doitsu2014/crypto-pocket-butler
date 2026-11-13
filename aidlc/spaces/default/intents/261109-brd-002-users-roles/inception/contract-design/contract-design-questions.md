# Contract Design — Questions

This stage pins the formal contracts across every boundary: the inter-unit edges from the dependency DAG and the gateway's `/v1` surface consumed outside the backend.

Answer each question with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [architect] Which boundaries should get a formal contract?

- A. Every edge in the unit DAG plus the gateway `/v1` surface
- B. Only the gateway `/v1` surface (external) and the service-to-service edges that carry security data (Support Access, Audit)
- C. Only the external `/v1` surface
- X. Other (please specify)

[Answer]: A — Every edge in the unit DAG plus the gateway `/v1` surface.

## Q2. [architect] Which integration mechanism applies?

The system uses synchronous internal HTTP with no message queues; the browser reaches services only through the gateway.

- A. REST/JSON with RFC 7807 problem bodies on every boundary; audit events go over HTTP
- B. REST/JSON plus an event contract (AsyncAPI) for audit events
- C. REST/JSON only for external, shared-schema contracts internally
- X. Other (please specify)

[Answer]: A — REST/JSON with RFC 7807 problem bodies on every boundary; audit events over HTTP.

## Q3. [architect] What versioning and breaking-change policy should apply?

- A. A `/v1` prefix; additive changes only within v1; a breaking change needs `/v2` and a deprecation window
- B. No version prefix; additive changes only, with a breaking change announced in the same release
- C. Date-based versioning per endpoint
- X. Other (please specify)

[Answer]: A — A `/v1` prefix; additive changes only within v1; a breaking change needs `/v2` and a deprecation window.

## Q4. [architect] What error, timeout, and retry behaviour should every boundary share?

- A. RFC 7807 `ProblemDetails` everywhere; 401 for missing/invalid tokens, 403 for role denial, 404 for undeclared routes and non-disclosure; connect timeout 5s, read timeout 10s; bounded retries (3 attempts, exponential backoff with jitter) only on idempotent calls carrying an idempotency key
- B. The same errors, with a 30s read timeout and 5 retries everywhere
- C. Per-service timeouts and retry counts, agreed case by case
- X. Other (please specify)

[Answer]: A — RFC 7807 `ProblemDetails` everywhere; 401/403/404 as specified; 5s connect and 10s read timeouts; 3 retries with exponential backoff and jitter, only on idempotent calls carrying an idempotency key.

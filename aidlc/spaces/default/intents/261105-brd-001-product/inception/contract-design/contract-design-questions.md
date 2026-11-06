# Contract Design — Questions

The unit DAG names the boundaries; these questions settle how they are specified. Answer each
with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [Architect] What format should the HTTP contracts use?

- A. OpenAPI 3.1 in YAML for every boundary, including the internal ones
- B. OpenAPI for the public gateway API only; internal boundaries described as shared Rust types
- C. Prose contracts only
- X. Other (please specify)

[Answer]: A — OpenAPI everywhere

## Q2. [Architect] How should contracts be versioned?

- A. URL path version (`/v1/...`) from day one; breaking changes ship a new major version
- B. Header-based versioning
- C. No versioning before 1.0; additive changes only
- X. Other (please specify)

[Answer]: A — URL path /v1

## Q3. [Architect] What error shape should every contract use?

- A. RFC 7807 `application/problem+json` with `type`, `title`, `status`, `detail`, and `correlationId`
- B. A custom `{ code, message, details, correlationId }` envelope
- C. Bare HTTP status codes with a plain-text body
- X. Other (please specify)

[Answer]: A — RFC 7807

## Q4. [Architect] Is the gateway API public or internal?

- A. Internal and documented for the SPA and Support Console only; no third-party consumers
- B. Public with API keys for future integrations
- C. No contract for the gateway; the SPA and gateway are developed together
- X. Other (please specify)

[Answer]: A — Internal only

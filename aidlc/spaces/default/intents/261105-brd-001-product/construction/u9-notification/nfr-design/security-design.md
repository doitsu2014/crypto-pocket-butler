# Security Design — u9-notification

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- **Auth**: JWT (owner-only) for feed/preferences; per-caller service tokens (Identity, Account,
  Portfolio, Reporting) for C9, Identity for erasure; C11 run token.
- **Address**: read from C3 `GET /v1/users/{userId}/contact` at delivery; never stored, logged, or
  traced.
- **Content**: per-type allow-lists; sanitisation drops emails/addresses/secrets and records
  `sanitisedFields`.
- **Audit**: C10 events for security-alert delivery/abandonment and mandatory-preference refusals,
  with the full schema fields and no payload value.
- **Erasure**: Identity calls C9 anonymise; userId is replaced with a random UUIDv4 opaque reference
  and no mapping to the user is retained.
- **Least privilege**: schema `notification`, role `notification_svc`; no cross-schema grant.
- **Transport**: TLS to provider and internally; Postgres sslmode=require.

NFR references: NFR9.1–NFR9.6.

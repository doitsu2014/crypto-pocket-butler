# Security Design — u8-reporting

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- **Auth**: JWT owner-only for C1 briefs; per-caller tokens for internal callers (Identity privacy),
  C11 run token; Reporting presents its outbound token to C3/C7/C8/C9/C10, each allow-list naming it
  (NFR9.7).
- **Content safety**: briefs carry figures and labels only; no credential, token, or full address.
- **Audit**: `reporting.brief.failed` and `reporting.brief.email_submitted` — C10's system allow-list
  must be extended to accept them (recorded follow-up; if not extended the run uses the allow-listed
  `system.reporting.brief` action with the specific outcome in context).
- **Privacy**: internal export/anonymise for Identity (userId + idempotency key); anonymise replaces
  userId with a UUIDv4 reference; Reporting is in Identity's contributor set.
- **Least privilege**: schema `reporting`, role `reporting_svc`; no cross-schema grant.
- **Transport**: TLS; Postgres sslmode=require.

NFR references: NFR9.1–NFR9.7.

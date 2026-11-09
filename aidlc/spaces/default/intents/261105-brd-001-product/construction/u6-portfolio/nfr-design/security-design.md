# Security Design — u6-portfolio

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- **Auth**: JWT owner-only for C1; per-caller service tokens for C8 (Reporting for snapshots, Identity for privacy export/anonymise), C11 (Job Orchestration); Portfolio presents its own token to C5/C6/C7/C9/C10 with each registry allow-list.
- **Backend math**: all valuation/allocation/drift in `application`/`domain`; the SPA receives
  numbers and provenance (NFR9.3).
- **Immutability**: `portfolio_svc` has SELECT/INSERT only on `valuation_snapshots` and no DELETE.
- **Validation**: name, base currency, ids, targets 0–100, guardrails > 0 with a target, intent
  quantity > 0; RFC 7807.
- **Audit**: recorded trade (`portfolio.trade.recorded`), target change (`portfolio.targets.changed`),
  guardrail change (`portfolio.guardrail.changed`), and intent cancellation
  (`portfolio.intent.cancelled`) write C10 with actor (user), action, target (portfolio/asset/intent),
  timestamp, context (ids only), and outcome; retried out of band.
- **Privacy**: C8 export (GET, userId) and anonymise (POST, userId + idempotency key, UUIDv4
  reference) for Identity's orchestration.
- **Least privilege**: schema `portfolio`, role `portfolio_svc`; no cross-schema grant, no RLS.
- **Transport**: TLS; Postgres sslmode=require.

NFR references: NFR9.1–NFR9.6, NFR13.1, NFR12.1.

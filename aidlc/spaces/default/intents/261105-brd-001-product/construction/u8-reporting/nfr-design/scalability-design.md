# Scalability Design — u8-reporting

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- API 2 replicas ceiling 4; worker 1 replica.
- ≈4.2M briefs/year; two-year retention ≈8.4M rows; sections JSONB ≈2 KB; prune nightly at 2 years.
- Run: 200 per batch, 8 batches × 4 in flight; pools 10/8.

NFR references: NFR20.1–NFR20.6.

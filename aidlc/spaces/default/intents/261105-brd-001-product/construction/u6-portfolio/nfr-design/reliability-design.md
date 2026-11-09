# Reliability Design — u6-portfolio

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- **Degraded valuation**: C5 unavailable → `partial` + `holdingsUnavailable`; C6 unavailable →
  unpriced lines; only Postgres failure is 503 (NFR1.2).
- **Snapshots**: immutable via grants; reproduction verifies digest; mismatch flagged (NFR1.3).
- **Run**: lease 900 s (the 15-minute run budget) renewed every 60 s to `now + 900 s`; a failed renewal stops the run; an expired lease is swept to failed; one retry after 10 min; missed run visible (NFR1.4).
- **Recorded trade**: C7 failure refuses the trade; audit after write with 3 attempts (1/2/5 s), a
  background task so the 900 ms budget is untouched; persistent failure pages (NFR1.5).
- **Dependencies**: C5 200 ms, C6 150 ms, C7 300 ms, C9 1/3/10 s ×3, C10 1/2/5 s ×3.
- **Health/backup**: `/healthz`, `/readyz` Postgres; RPO ≤24 h, RTO ≤4 h.

NFR references: NFR1.1–NFR1.7.

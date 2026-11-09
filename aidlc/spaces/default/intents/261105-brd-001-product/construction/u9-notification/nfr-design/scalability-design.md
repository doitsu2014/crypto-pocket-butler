# Scalability Design — u9-notification

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- API: 2 replicas, ceiling 4; pool 10 each.
- Worker: 1 replica for delivery and retention; pool 8.
- Volume: ≤ 500k alerts/month; ≤ 100k preference rows; ≤ 30 M alert/attempt rows over 12 months.
- Delivery: 200 per batch, concurrency 20, provider limit 240/min; the single worker holds a DB lease
  and is the only delivery executor (API replicas only write the wake flag; it polls every 5 s).
- Retention: nightly delete of alerts/attempts older than 12 months.

NFR references: NFR20.1–NFR20.6.

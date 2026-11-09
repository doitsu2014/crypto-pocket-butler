# Performance Requirements — u9-notification

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR2.1 | `POST /v1/alerts` acknowledgement | p95 ≤ 100 ms |
| NFR2.2 | Preference read/save | p95 ≤ 150 ms |
| NFR2.3 | Delivery run (1,000 due alerts) | ≤ 5 minutes |
| NFR2.4 | In-app feed read | p95 ≤ 200 ms |
| NFR2.5 | Mandatory security email handoff to the provider | p95 ≤ 30 s after submit: submit sets a durable wake flag that the single leased delivery worker polls every 5 s |

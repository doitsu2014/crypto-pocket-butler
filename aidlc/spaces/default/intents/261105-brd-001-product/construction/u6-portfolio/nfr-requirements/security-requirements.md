# Security Requirements — u6-portfolio

| ID | Requirement | Target |
|---|---|---|
| NFR9.1 | Transport | TLS internally and to C5/C6/C7/C9/C10 |
| NFR9.2 | Authorization | owner-only reads/writes; internal C8 callers (Reporting) use per-caller tokens; C11 run token; Portfolio presents its own service identity to C5/C6/C7/C9/C10 and each registry allow-lists it |
| NFR9.3 | Backend math | valuation, allocation, drift computed only in Portfolio; the SPA receives numbers and provenance |
| NFR9.4 | Audit | target/guardrail changes, recorded trades, intent cancellation write C10 with full fields |
| NFR9.5 | Input validation | portfolio name, base currency code, account/asset ids, target 0–100, guardrail > 0, intent quantity > 0; RFC 7807 |
| NFR9.6 | Least privilege | schema `portfolio`, role `portfolio_svc`; no cross-schema grant, no RLS |
| NFR12.1 | Supply chain | `cargo audit` and image scan before release |
| NFR13.1 | Privacy | C8 `POST /v1/internal/privacy/export` (userId) returns portfolios, targets, guardrails, intents, trades, and snapshot lines; `POST /v1/internal/privacy/anonymise` (userId + idempotency key) replaces the userId with a non-reversible reference while snapshots remain reproducible; Identity calls both |

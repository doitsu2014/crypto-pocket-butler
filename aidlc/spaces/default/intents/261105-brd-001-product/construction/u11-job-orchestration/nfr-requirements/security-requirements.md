# Security Requirements — u11-job-orchestration

| ID | Requirement | Measure |
|---|---|---|
| NFR9.1 | Every operator path validates the forwarded Supabase JWT against JWKS (signature, issuer, audience, expiry) in this service | Middleware tests for a bad signature, issuer, audience and expiry |
| NFR9.2 | The operator role comes from `app_metadata.role` and must be `admin` or `support`; the top-level `role` claim and `user_metadata.role` are not authority and never grant access | Handler test: a `user_metadata.role = admin` token is 403 |
| NFR9.3 | The target service token is read from a secret file and never logged, echoed, or stored in a run row | Secret-file loader test; log review |
| NFR9.4 | Every outbound call goes only to an allow-listed job name and a registered service URL; no user-supplied URL | Catalogue constant; unknown-job test |
| NFR9.5 | `detail`, `reason`, and audit context hold identifiers and statuses only: no credential, token, response body, or full address | Unit tests over the detail builder |
| NFR9.6 | Every operator action (manual run, backfill) is audited with actor, action, target, instant, and outcome | Audit adapter test |
| NFR9.7 | The inbound token registry allow-lists only this service's operations | Migration grants and seeds |
| NFR9.8 | All production traffic uses TLS; the service holds no data beyond run records | Stack overlay check |

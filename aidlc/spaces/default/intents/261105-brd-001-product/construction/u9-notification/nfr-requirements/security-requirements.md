# Security Requirements — u9-notification

| ID | Requirement | Target |
|---|---|---|
| NFR9.1 | Transport | TLS internally and to the email provider |
| NFR9.2 | Authorization | owner-only preference reads/writes; internal C9 callers use per-caller service tokens (Identity, Account, Portfolio, Reporting); C11 run token |
| NFR9.3 | Address handling | the verified address is read from C3 at delivery time and never stored or logged |
| NFR9.4 | Content safety | no credential, token, seed, or full address in any channel, log, or delivery record |
| NFR9.5 | Audit | security-alert delivery/abandonment and mandatory-preference refusals write C10 with actor, action, target (alert type/id), timestamp, context (channel, sanitised field names), and outcome |
| NFR13.1 | Erasure | Identity calls Notification `POST /v1/internal/privacy/anonymise` with a service token, userId, and idempotency key; Notification replaces the userId on the user's alerts and delivery attempts with a non-reversible reference and keeps the twelve-month operational rows |
| NFR9.6 | Least privilege | schema `notification`, role `notification_svc`; no cross-schema grant, no RLS |
| NFR12.1 | Supply chain | `cargo audit` and image scan before release |

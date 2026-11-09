# Security Requirements — u8-reporting

| ID | Requirement | Target |
|---|---|---|
| NFR9.1 | Transport | TLS internally and to C3/C7/C8/C9/C10 |
| NFR9.2 | Authorization | owner-only brief reads; internal callers use per-caller tokens (C3 due-users, C7 P&L, C8 reads, C9 alerts, C10 audit); C11 run token |
| NFR9.3 | Content safety | briefs never include credentials, tokens, or full addresses |
| NFR9.4 | Audit | `reporting.brief.failed` and `reporting.brief.email_submitted` write C10 with actor (system), action, target (briefId), timestamp, context (period, channel), outcome; actions are on C10's system allow-list |
| NFR9.5 | Privacy | Reporting exposes internal export/anonymise (see the contract's C1 Reporting privacy row); Identity's service token authorizes both and Reporting is in Identity's deletion contributor set |
| NFR9.7 | Outbound registries | Reporting's outbound token is allow-listed by Identity (due users), Ledger (P&L), Portfolio (portfolios/valuation), Notification (alerts) and Audit |
| NFR9.6 | Least privilege | schema `reporting`, role `reporting_svc`; no cross-schema grant |
| NFR12.1 | Supply chain | `cargo audit` and image scan before release |

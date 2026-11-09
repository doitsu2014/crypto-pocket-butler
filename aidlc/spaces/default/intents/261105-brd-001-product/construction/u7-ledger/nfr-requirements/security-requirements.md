# Security Requirements — u7-ledger

| ID | Requirement | Target |
|---|---|---|
| NFR9.1 | Transport | TLS on the internal overlay and every external call |
| NFR9.2 | Authorization | owner-only reads; internal callers (Portfolio, Reporting) use per-caller service tokens with an allow-list; corrections and recorded trades require the owner |
| NFR9.3 | Append-only enforcement | the runtime role has SELECT/INSERT only on transactions and corrections; UPDATE/DELETE are not granted |
| NFR9.4 | Audit | corrections and recorded trades write C10 events with actor, transaction, field, old/new value, outcome, and a reason **hash** (never the free text); no credential |
| NFR9.5 | Input validation | quantity > 0, price ≥ 0, kind enum, executedAt not more than 300 s in the future (venue clock skew tolerance; later values are clamped), assetId exists (C4); RFC 7807 field-level problems |
| NFR13.1 | Privacy | C7 exposes export and anonymise for a user's ledger: export returns the user's transactions and corrections, anonymise replaces the user id with a non-reversible reference while keeping the financial-retention rows; identity orchestration calls it |
| NFR9.6 | Least privilege | schema `ledger`, role `ledger_svc`; no cross-schema grants, no RLS |
| NFR12.1 | Supply chain | `cargo audit` and image scan before release |

## Notes

- Ledger stores no credential, wallet address, or personal data beyond the user id needed for
  ownership.
- A correction reason is stored on the correction row and in the audit event; it is user text and
  must never contain a key or token (validated at the boundary).

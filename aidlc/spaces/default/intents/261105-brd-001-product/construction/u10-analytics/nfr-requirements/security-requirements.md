# Security Requirements — u10-analytics

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR9.1 | Transport | TLS for the internal overlay and every external call |
| NFR9.2 | Ingest identity | The gateway validates the JWT and sets `X-Auth-Subject`; Analytics trusts it only from the gateway and calls C3 with it |
| NFR9.3 | PII exclusion | Allow-listed properties only; email/address/IP/secret patterns dropped or refused; no credential, address, or raw id stored |
| NFR9.4 | Pseudonymisation | HMAC-SHA256 with a 32-byte Vault salt; rotated only on suspected compromise, not on a schedule; every version kept 12 months; erasure hashes the raw id under every retained version. A compromise rotation re-buckets flags once and splits cohort joins at the version boundary; both are accepted and recorded |
| NFR9.5 | Audit | C10 events for flag create/update/delete and for a refused ingest category; no PII in context |
| NFR9.6 | Authorization | Flag reads need an authenticated user; aggregates and flag administration need the `admin` role; erase needs Identity's service token |
| NFR9.7 | Least privilege | One schema `analytics`, one role `analytics_svc`; no cross-schema grants, no RLS |
| NFR12.1 | Supply chain | `cargo audit` and image scan before release |

## Notes

- Data classification: pseudonymous product telemetry. The HMAC salt is a Vault secret; losing it
  breaks erasure, so it is backed up with the Vault.
- The erase endpoint refuses any caller other than Identity (403).

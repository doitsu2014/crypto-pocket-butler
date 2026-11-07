# Security Requirements — u5-market-data

Inputs: `functional-design/functional-spec.md` (BR5), `inception/requirements-analysis/requirements.md` (BRD-012), and C6/C10/C11.

| ID | Requirement | Target |
|---|---|---|
| NFR9.1 | Transport | TLS for every external call and the internal overlay; no plaintext provider call |
| NFR9.2 | Internal caller authorization | C6 uses per-caller service tokens (Portfolio, Reporting, Job Orchestration) with a per-operation allow-list; C11 uses the shared internal run token from Vault. Each token is stored as a SHA-256 hash in `market_data.service_token_registry` |
| NFR9.3 | No secret in logs or errors | Provider keys never logged, echoed in a problem document, or written to an observation; source errors are categorised |
| NFR9.4 | Audit | C10 events for failed refresh runs, with actor (system), action, target, occurredAt, context (run id and source name), outcome; source-registry changes are audited by U1 |
| NFR9.5 | Provider credentials | Held in Vault; read through a `SECURITY DEFINER` function in the `market_data` schema granted to `market_data_svc`, so no cross-schema table grant exists; no env-file or source fallback |
| NFR9.6 | Input validation | `assetIds` non-empty and ≤ 1,000; currency codes three letters; RFC 7807 problems with a correlation id and a declared 4xx in C6 |
| NFR9.7 | Least privilege | The Market Data Postgres role reads and writes only its own schema; no cross-schema grant, no RLS |
| NFR12.1 | Supply chain | `cargo audit` and container-image scan before release, per BRD-012 NFR-SEC-14 |

## Notes

- Data classification: public market data; no personal data is stored.
- The manual refresh path is authorized with the Job Orchestration token; no separate operator
  identity exists for the C11 route.

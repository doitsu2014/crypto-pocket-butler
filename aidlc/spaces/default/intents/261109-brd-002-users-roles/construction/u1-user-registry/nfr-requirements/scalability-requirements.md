# Scalability Requirements — u1-user-registry

**Upstream:** `functional-spec.md`, `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR20.1 | User base | 1 million users without schema change; indexed lookup by subject hash and internal id | BRD-001 NFR20 |
| NFR20.2 | Throughput | 500 principal resolutions per second per replica at the p95 target; horizontal scaling adds linearly | BRD-001 NFR20 |
| NFR20.3 | Write load | One row write per new user; provisioning is idempotent under retry | BR1.1 |
| NFR20.4 | Search | Admin lookup by exact email or id uses an index; no full-table scan | AC7.1.1 |

The unit is stateless beyond Postgres; replicas share the same schema and role.

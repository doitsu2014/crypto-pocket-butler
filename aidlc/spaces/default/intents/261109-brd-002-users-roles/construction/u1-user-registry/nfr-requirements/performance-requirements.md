# Performance Requirements — u1-user-registry

**Upstream:** `functional-spec.md` (W1–W4), `rules.md` (BR1.1–BR1.10), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR2.1 | Principal resolution | p95 ≤ 100 ms server-side for an existing user, excluding the audit write; the audit write gets 300 ms with one retry | BRD-001 NFR2; C3, C9 |
| NFR2.2 | First provisioning | p95 ≤ 2 s including the C8 gate call (its own 1.5 s cap) and the audit write; the registry itself adds ≤ 500 ms | C3, C8 |
| NFR2.3 | Admin lookup | p95 ≤ 300 ms over 1 million users | AC7.1.1 |
| NFR2.4 | Profile update | p95 ≤ 100 ms | W2 |

Lookups are indexed by the subject hash and by the internal id; the admin lookup uses an indexed exact-match on email or id, not a table scan.

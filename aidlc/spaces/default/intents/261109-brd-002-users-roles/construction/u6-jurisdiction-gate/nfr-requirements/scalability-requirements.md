# Scalability Requirements — u6-jurisdiction-gate

**Upstream:** `functional-spec.md` (W1–W3), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR20.1 | List size | Up to 300 country entries held in memory per replica | BRD-001 NFR20; W3 |
| NFR20.2 | Check throughput | 200 checks/s per replica at the p95 target; horizontal scaling adds linearly | BRD-001 NFR20 |
| NFR20.3 | Provider quota | 600 provider calls per minute per replica; the per-caller decision cache is keyed by (country code, salted IP hash) with a 5-minute TTL and holds no raw IP | NFR20.2; W2 |
| NFR20.4 | Write load | One database write per decision at most (the audit event); the list is read-only between publishes | W1, W2 |
| NFR20.5 | Replica refresh | Each replica re-reads the active list version every 10 s; the maximum staleness after a publish is 10 s | NFR20.1; W3 |
| NFR20.6 | Growth | The design holds at ten times the launch list size with no change to the stored model | NFR20.1 |

Scaling is horizontal: the gate is stateless apart from the versioned list, which every replica loads from the same store and refreshes on a fixed interval because there is no message queue to broadcast a publish.

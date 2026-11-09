# Logical Components — u9-notification

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Notification API | C9 submit, feed/preferences, erasure | 2 replicas | One task |
| Dedupe | idempotency key check | In-process/DB | Duplicate returns deduplicated |
| Content guard | allow-lists + sanitisation | In-process | Drops fields, records them |
| Delivery worker | batches, retries, polls the durable wake flag every 5 s | 1 leased replica | Backlog visible |
| `notification` schema | preferences, alerts, attempts | Supabase | Service-owned |
| C3 client | verified contact at delivery | Per delivery | Transient retried; missing is permanent |
| C10 client | audit | Per event | Retried then paged |

## Boundaries

- No address stored; no payload value logged.
- No queue; alerts are the queue.
- Erasure via Identity's call.

NFR references: NFR20.1, NFR9.3, NFR1.4.

# Problem details

Every service renders a refusal the same way, and the SPA depends on that sameness. This is the
shape to copy, not to redesign.

## The body

`application/problem+json`, RFC 7807, one struct per service in `services/<name>/http/src/problem.rs`
(the near-duplication across services is deliberate — each service owns its own boundary types):

```json
{
  "type": "https://errors.cpb/identity/<kind>",
  "title": "short human-readable summary",
  "status": 422,
  "detail": "when a human-readable explanation helps",
  "errors": [{ "field": "email", "reason": "is required" }],
  "correlationId": "..."
}
```

- `type` is a stable URI under `https://errors.cpb/<service>/<kind>`; treat the string as a
  contract, not as prose to reword.
- `errors` carries **field paths and reasons only — never the rejected value**.
- `correlationId` comes from the `x-correlation-id` request header (generated when absent) and is
  repeated on the response, so a support screen and a log line can be joined.
- One pinned exception in Identity: `NO_RECORD_TYPE` (`…/problems/no-record`) marks "the caller has
  no open request". The SPA treats only that type as an absence and every other `404` as an error,
  so it is deliberately outside the normal family.

## How a typed error becomes this body

```
domain::DomainError        ← business refusals, no transport knowledge
   ↓  AppError::from_domain
application::AppError      ← the use-case error, still transport-free
   ↓  AppError::from_repository / from_external
http::problem              ← status + type + title + detail + correlation id
```

`AppError` is constructed only through `from_domain`, `from_repository` and `from_external`. Errors
are surfaced or logged, never swallowed; distinguishing recoverable from fatal happens at the
application boundary, not at the HTTP one.

## Status conventions in use

| Situation | Status |
|---|---|
| Validation refusal (bad field, bad id) | `400`/`422` with `errors[]` |
| Jurisdiction gate refusal | `422` (C8 contract) |
| Not found, or an admin lookup outside the caller's reach | `404` (reach is not disclosed by a `403`) |
| Unauthenticated / expired token | `401` |
| Insufficient role | `403` |
| A rule refuses a legal-looking transition (e.g. erasing an `active` user) | `409` |
| Rate budget exhausted | `429` |

## What never goes in a problem body

No secret, token, key, DSN, full address, or another user's data. `detail` may name a field and a
rule; it may not echo the input that failed.
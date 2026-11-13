# Monitoring Design — u4-preferences-consent

**Upstream:** `observability-design.md`, `infrastructure-specification.md`, `reliability-design.md`.

## The collection gap, stated first

As with u2, no Prometheus service or scrape configuration is declared in the repository. The
metrics below are emitted at Identity `/metrics`; the scrape job that collects them is:

```yaml
- job_name: identity-api
  metrics_path: /metrics
  static_configs:
    - targets: ["identity-api:8083"]
```

Until that platform work lands, every alert below is defined but not armed.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `consent_state_queries_total` | Identity `/metrics` | — | Gate volume |
| `consent_state_queries_with_missing_total` | Identity `/metrics` | step change in `has_missing="true"` | A policy or UI change is blocking users |
| `consent_state_duration_seconds` | Identity `/metrics` | p95 above 40 ms | Guards the gate budget |
| `consent_events_total` | Identity `/metrics` | — (by kind and action) | Consent activity |
| `preference_writes_total` | Identity `/metrics` | refused > 5% | A client sends invalid values |
| `consent_withdrawal_rollbacks_total` | Identity `/metrics` | any non-zero | A withdrawal was refused because its audit intent could not be recorded |
| `consent_outbox_pending` | Identity API `/metrics` | — | Backlog size (the drainer runs in `identity-api`, not `identity-worker`) |
| `consent_outbox_oldest_age_seconds` | Identity API `/metrics` | > 15 min | Backlog age, emitted in the request path's metrics too — see the last-pass alert below |
| `consent_outbox_last_pass_timestamp_seconds` | Identity API `/metrics` | `time() - value > 600` | **The wedge detector.** A registered gauge keeps its last value while the process is scraped, so a stopped drainer leaves a *frozen* gauge, not an absent one; the timestamp going stale is what actually fires |
| `consent_outbox_deliveries_total` | Identity API `/metrics` | any `dead` outcome | A bad payload or an auth failure against C9 |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Outbox backlog | oldest age > 15 min | P1 (page) | Identity on-call — audit intents are not reaching C9 |
| Drainer wedged | `time() - consent_outbox_last_pass_timestamp_seconds > 600` | P1 (page) | Identity on-call — the drainer has stopped passing; a frozen gauge is not an absent one, so this is the rule that catches it |
| Audit delivery fast burn | > 1% of pending rows older than 5 minutes for 5 minutes | P2 (ticket) | Identity on-call — fires inside the 5-minute SLO window, before the 15-minute page |
| Audit delivery dead | any `dead` delivery over 5 minutes | P1 (page) | Identity on-call — check the payload and the C9 token |
| Withdrawal refused on outbox failure | any `consent_withdrawal_rollbacks_total{cause="outbox_failed"}` over 5 minutes | P1 (page) | Identity on-call — preferences writes are failing |
| Consent-state errors | > 1% over 5 minutes | P1 (page) | Identity on-call — connections are being refused |
| State answers with a missing acceptance | step change (> 3× the 7-day baseline) | P2 (ticket) | Product/Identity |
| State-query latency | p95 above 40 ms for 10 minutes | P2 (ticket) | Identity team |
| Preference refusals | > 5% of writes over 30 minutes | P2 (ticket) | Identity team / the offending client |

## SLIs / SLOs

| SLI | Target | Window |
|---|---|---|
| State query answered successfully | 99.9% | 30-day rolling |
| Accepted withdrawal has its outbox row | 100% | 30-day rolling |
| Audit delivery lag | 99.9% within 5 minutes | 30-day rolling |
| Consent-event durability | 100% | 30-day rolling |

## Logs & tracing

Structured JSON to stdout, collected by Loki; every line carries `service`, `trace_id`,
`correlation_id`. The withdrawal is one span (advisory lock, read, insert, outbox insert); the
drainer's post is a span with the outbox id and outcome. No names, emails, subjects, tokens, or
wallet addresses.

## Dashboards

One Grafana board: state queries and the `has_missing` split, consent events by kind and action,
preference writes and refusals, state latency, outbox pending/oldest-delivery by outcome.

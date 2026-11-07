# Reliability Requirements — u5-market-data

Inputs: `functional-design/rules.md` (BR1, BR4), `inception/requirements-analysis/requirements.md` (NFR5), and C6/C11.

| ID | Requirement | Target |
|---|---|---|
| NFR1.1 | Price availability | 99.9% of priced-or-stale quotes over 30 days; the SLI is per asset-quote in a `getPrices` batch, and "covered" means the asset has at least one accepted observation in the last 24 hours |
| NFR1.2 | Freshness SLO | Over a 30-day rolling window, 99% of top-100 quotes live within 5 minutes and 99% of tail quotes live within 15 minutes |
| NFR1.3 | Single flight | At most one run per job (top100, tail); each has a 300-second lease renewed every 60 seconds; an expired lease is swept and the run marked failed |
| NFR1.4 | Selection and fallback | The read path selects from stored observations by source priority; the refresh stores the first accepted observation per asset, falling back to the next source at refresh time |
| NFR1.5 | Retry policy | Timeouts, 5xx, and 429 retried 3 times with jitter; other 4xx are permanent for the run |
| NFR1.6 | Data integrity | A rejected reading never replaces an accepted observation; zero or negative prices never stored; an observation whose `observedAt` is more than 300 seconds ahead of `receivedAt` is rejected |
| NFR1.7 | Backup and recovery | Supabase backup regime; RPO ≤ 24 h, RTO ≤ 4 h; restore rehearsal before launch |
| NFR1.8 | Health | `/healthz` liveness; `/readyz` checks Postgres; the Swarm health check gates rolling updates |
| NFR1.9 | Decimal precision | Prices and rates parsed, stored, and served as decimal strings; no binary floating point anywhere |
| NFR1.10 | FX availability | 99.9% of `getFxRate` requests return a fresh direct or derived rate over 30 days |
| NFR1.11 | C4 unavailability on the read path | When the source list cannot be read, serve the newest accepted observation per asset as `stale` with its recorded source, skipping priority selection; an asset with no observation is `unpriced` in the same 200 response. 503 is reserved for Postgres unavailability |

## Notes

- The service degrades to stale prices rather than failing the read path; `stale` is an explicit
  state.
- An unreachable C4 during a refresh fails the run rather than pricing from an unknown source set;
  previous observations keep being served.
- The top100 job completes within 3 minutes on a 5-minute cadence and the tail job within 10
  minutes on a 10-minute cadence, which keeps NFR1.2 reachable with margin.

# Logical Components — u4-account

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Account API | C1 account routes, C5 holdings/transactions, privacy export/anonymise, C11 run endpoint | 3 stateless replicas | One task; gateway routes around |
| `account-worker` | Venue/RPC sync, streaming CSV import, snapshot write, lease | 1 replica | One task; lease re-claimed |
| Account Postgres schema | Account, Credential, WalletConnection, SyncRun, HoldingSnapshot, ImportBatch, ImportedTransaction, ImportRowError | Supabase managed | Service-owned schema |
| Vault | Credential secret references | Supabase managed | External; key read fails the provider call, never falls back |
| Audit client | C10 events | per request | External; security events surfaced |
| Notification client | C9 alerts | per event | External; bounded retry |
| Asset Registry client | C4 resolve and supported chains | per call | External; a failed resolve fails the run |
| Identity client | C3 grant check | per support read | External; a failed check refuses the read |
| Auth admin client | C15 claim maintenance | per credential event | External; retried, recorded |

## Boundaries

- No provider response, credential, or address is logged.
- The API and worker share the schema but not a process.
- No other unit reads the Account schema directly; Portfolio, Ledger, and Identity call C5.

## Blast radius

- A failed API task affects its replica only.
- A failed sync affects one account; a failed provider affects that venue's accounts only.
- A compromised user JWT exposes that user's accounts and holdings only.
- A failed Vault read fails credential use and the sync; no plaintext fallback exists.

## Shared resources

- Postgres pool (12/replica, 8 worker), the Audit service, the gateway rate limiter, and the
  observability stack.

NFR references: NFR1.1, NFR20.1, NFR9.1.

# Risk and Sequencing Rationale — Crypto Pocket Butler

## Heuristic

Two heuristics, applied in order:

1. **Walking skeleton first** (Cockburn): the first Bolt is the smallest end-to-end slice that
   proves the architecture connects, before feature breadth.
2. **WSJF** (Reinertsen / SAFe): after the skeleton, order Bolts by (value + urgency + risk
   reduction) ÷ size, subject to the unit dependency graph.

The human chose "thin slice first" and "use WSJF" at the planning interview, and named **data
correctness** as the biggest worry — which is why the ledger and portfolio Bolts carry high
risk-reduction scores.

## Scores

| Bolt | Unit | Value | Urgency | Risk reduction | Size | WSJF |
|---|---|---|---|---|---|---|
| B1 | U1 walking skeleton | 9 | 10 | 10 | 13 | 2.23 |
| B2 | U12 audit | 5 | 4 | 8 | 3 | 5.67 |
| B3 | U3 identity | 8 | 8 | 7 | 5 | 4.60 |
| B4 | U13 support console | 4 | 3 | 4 | 3 | 3.67 |
| B5 | U5 market data | 8 | 7 | 8 | 5 | 4.60 |
| B6 | U4 account | 9 | 8 | 8 | 8 | 3.13 |
| B7 | U10 analytics | 4 | 3 | 3 | 3 | 3.33 |
| B8 | U9 notification | 6 | 5 | 5 | 5 | 3.20 |
| B9 | U7 ledger | 9 | 7 | 9 | 8 | 3.13 |
| B10 | U6 portfolio | 10 | 9 | 8 | 8 | 3.38 |
| B11 | U8 reporting | 6 | 5 | 4 | 5 | 3.00 |
| B12 | U11 job orchestration | 5 | 5 | 6 | 5 | 3.20 |
| B13 | U2 web app | 9 | 8 | 6 | 8 | 2.88 |

Scores are relative within this plan, not absolute.

## Why this order

- **B1 first despite the lowest score.** The walking skeleton is a deliberate exception to WSJF:
  it retires the largest architectural unknown — that the JWT path, schema and role isolation,
  the Swarm deployment, and the observability wiring all work together — before any feature
  commits to them.
- **B2 audit second.** Every later Unit emits security-relevant events; audit is cheap and
  unblocks all of them.
- **B3 identity before the domain services.** Roles, consent, and preferences are read by the
  account, notification, analytics, and web app units.
- **B4 support console early.** It only needs U1, U3, and U12, and it proves the least-privilege
  boundary before customer data fills the system.
- **B5 market data and B6 account in parallel.** Both are foundational inputs to valuation, and
  they have no dependency on each other.
- **B9 ledger before B10 portfolio.** Portfolio reads P&L from Ledger; correctness of cost basis
  is the human's stated biggest risk, so it is proven before valuation depends on it.
- **B13 web app last.** It consumes nine services; building it earlier would test against stubs
  and rework on every contract change.

## Deviations from topological order

None. Every Bolt respects the unit dependency graph in
`units-generation/unit-of-work-dependency.md`. The parallel groups in `bolt-plan.md` are pairs
and triples of Units with no dependency between them.

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Data correctness — decimals, cost basis, valuation | Medium | Critical | TDD on the ledger and portfolio rules; `rust_decimal` only; reconciliation tests against the ledger; worked examples in the requirements |
| Credential security — key leakage or misuse | Low | Critical | Vault encryption; read-only verification; no credentials in responses, logs, or audit; ownership checks on every read |
| Cross-user leakage | Low | Critical | Ownership checks in every service; explicit cross-user acceptance tests; audit of denials |
| Provider reliability — stale prices, failed syncs | High | High | Multi-source priority and fallback; freshness labels; sync status; missed-run alerts within five minutes |
| Scope drift from the BRDs (billing removed, coverage narrowed) | Medium | Medium | The requirements record the decisions; domain design and contracts follow them |
| Contract gaps found in review | High | Medium | Fix the gateway and provider contracts before B13; the open questions in `contract-summary.md` name what blocks which unit |
| Walking skeleton reveals a boundary problem | Low | High | B1 ships before feature breadth; if it fails, revisit Units Generation before continuing |

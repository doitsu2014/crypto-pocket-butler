# Global Readiness Scorecard

**Scope:** specification readiness for a **global mass-market retail** crypto portfolio product. This score measures whether the BRD set defines the right problem, requirements, rules, acceptance criteria, and decisions. It does **not** measure implementation, legal sign-off, or market validation.

## Result

| # | Dimension | Before (2026-11-04 review) | After (this revision) | Evidence |
|---|-----------|:--------------------------:|:---------------------:|----------|
| 1 | Core value loop: aggregation, valuation, P&L, allocation, orders, reporting | 5 | 9 | 004, 005, 006, 007, 008, 009, 013 |
| 2 | Coverage: exchanges, chains, wallets, imports | 3 | 8 | 004 coverage matrix, 005 sources, 014 paths |
| 3 | Global currency, localization, timezone | 1 | 8 | 005 FX, 007 base currency, 018 |
| 4 | Onboarding, education, activation | 1 | 8 | 014 |
| 5 | Notifications and retention loop | 1 | 8 | 015, 009 |
| 6 | Trust, security, privacy, legal, regulatory | 4 | 8 | 003, 012, 017, 019 |
| 7 | Monetization, entitlements, quotas | 1 | 8 | 016 |
| 8 | Support, status, incident communication | 1 | 8 | 019 |
| 9 | Performance, benchmarks, attribution | 1 | 8 | 020 |
| 10 | Measurement, analytics, accessibility | 2 | 8 | 018, 021 |
| — | **Overall specification coverage** | **4** | **8** | All BRDs, DECISIONS, TRACEABILITY |

Scores are directional, not statistically derived. The target for this revision was **8/10 specification coverage**; 10/10 would require implementation evidence and external sign-offs.

## What “8/10” means

An 8/10 BRD set:

- defines every major retail value loop with functional requirements, business rules, and acceptance criteria;
- resolves cross-document contradictions in a decision log;
- states global requirements — currency, locale, timezone, privacy, billing, support — as first-class;
- distinguishes Must/Should/Could and out-of-scope explicitly;
- identifies dependencies on architecture, legal, data licensing, and operations;
- leaves only decisions that genuinely require market validation or external counsel.

It does **not** mean the product is ready to launch.

## Dimension criteria

| Dimension | What earns 8/10 |
|-----------|-----------------|
| Core value loop | Holdings → valuation → P&L → allocation/drift → action → report, each with FR/BR/AC and data-quality labels |
| Coverage | Named launch venues and chains, regional availability, key verification, WalletConnect/xpub/CSV, spam handling |
| Global currency | User-selected base currency, FX series, historical FX, locale/timezone, localized legal docs |
| Onboarding | Demo, address, WalletConnect, CSV, exchange paths; education; activation events; measurable time to value |
| Notifications | Channels, catalogue, preferences, quiet hours, dedupe, mandatory alerts, delivery logs |
| Trust and law | MFA/passkeys, read-only verification, support access, privacy rights, jurisdiction gates, licensing review |
| Monetization | Plans, entitlements, quotas, merchant of record, tax, dunning, cancellation, app-store compatibility |
| Support | Help centre, tickets, SLAs, support access, status page, severity model, postmortems |
| Performance | TWR/MWR, benchmarks, contribution, attribution, methodology disclosure, data-quality labels |
| Measurement | Metric tree, event taxonomy, consent, feature flags, experiments, WCAG 2.2 AA |

## Remaining gaps to 10/10

These are **implementation, external, or validation** gaps, not missing BRD domains:

| Gap | Why it remains | Owner |
|-----|----------------|-------|
| Architecture for ledger, notifications, billing, analytics, support tooling | BRDs define the need; C4 containers and data ownership must be designed | Architecture |
| Launch-market selection and legal opinions | Requires counsel and market data; D-011 sets the strategy, not the list | Leadership/Legal |
| Market-data and benchmark licences | Commercial negotiation; D-020 blocks unlicensed sources | Legal/Data |
| Pricing validation | Hypothesis in BRD-016; needs research and beta data | Product/Finance |
| Exchange coverage contracts and regional availability | Commercial and technical validation per venue | Product/Data |
| FR-to-AC matrix per BRD | Goal-level trace exists; per-FR evidence is completed during backlog refinement | Product/QA |
| Operational readiness | On-call rota, runbooks, pen test, restore drills, status-page ownership | Operations |
| Native apps | Post-MVP; PWA is the launch mobile path | Product |
| DeFi/staking position accounting | Detected and labelled, not fully valued | Data/Product |
| Execution, all-asset aggregation, tax filing, KYC/AML | Explicitly post-MVP or never | Product/Legal |

## Verification checklist

- [x] Every anchor goal in BRD-001 traces to at least one BRD and requirement set.
- [x] The retail value loop — see all, know P&L, get alerted, act, report — is covered end to end.
- [x] Cross-BRD contradictions are resolved or logged in `DECISIONS.md`.
- [x] Global currency, locale, timezone, privacy, billing, support, and accessibility are specified.
- [x] Coverage is expressed as a per-region matrix, not a marketing claim.
- [x] Out-of-scope items are explicit and have a post-MVP path.
- [ ] Per-FR acceptance-evidence matrix completed for every Must requirement.
- [ ] Architecture impact assessed and new containers/external systems modelled.
- [ ] Legal, licensing, pricing, and launch-market decisions closed.
- [ ] Operational readiness exercises completed.

The unchecked items are the next programme of work; they do not reopen the BRD domains.

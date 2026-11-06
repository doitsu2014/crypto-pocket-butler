# Requirements Analysis — Questions

The BRDs answer most of what this stage needs. These questions settle the points the BRDs
leave open, so the requirements can be written without guessing. Answer each with a letter
after `[Answer]:` (for example `[Answer]: A`), or pick `X` and say what you want instead.

## Q1. [Product manager] Which markets and languages launch first?

BRD-001 leaves this open (OQ-001). The scope document proposes EU (MiCA) and UK, English (EU)
and English (UK).

- A. EU (MiCA) and UK first, English (EU) and English (UK), as the scope document says
- B. EU and UK plus the US in English at launch
- C. Global English launch, with jurisdiction gating per market
- X. Other (please specify)

[Answer]: X — EN + VN (Vietnam)

## Q2. [Product manager] Which exchanges and chains are in the launch coverage matrix?

BRD-001 requires at least five exchanges, ten chains, and three import methods (OQ-003), but
does not name them here.

- A. Five exchanges and ten chains as the backlog scopes them; the exact list comes from the registry BRD (BRD-010)
- B. Fewer exchanges and chains at launch, expanded after
- C. More than five exchanges at launch
- X. Other (please specify)

[Answer]: X — Binance, OKX (2 exchanges) + 3 chains (EVM, BEP20, SOL)

## Q3. [Product manager] Is the Free/Pro packaging committed or still a hypothesis?

BRD-001 marks the packaging a draft hypothesis (OQ-002); BRD-016 defines how plans and quotas
are enforced server-side.

- A. Keep it a hypothesis — require enforcement and entitlements, not specific prices
- B. Commit the Free/Pro tiers and prices now
- C. One paid tier only at launch
- X. Other (please specify)

[Answer]: X — Now is open source (free)

## Q4. [Product manager] Is native mobile a first-year commitment?

BRD-001 marks native iOS/Android post-MVP (OQ-005) and makes the installable PWA the primary
mobile path.

- A. PWA only in the first year; native stays post-MVP
- B. Native iOS/Android is a first-year commitment
- C. Decide after the beta
- X. Other (please specify)

[Answer]: C — Decide after beta

## Q5. [Compliance specialist] How long do we keep financial records and audit logs?

BRD-012 leaves retention open (OQ-003). The scope document proposes five years for financial
records and twelve months for audit logs.

- A. Five years for financial records, twelve months for audit logs, as proposed
- B. Shorter: two years for financial records, six months for audit logs
- C. Decide per jurisdiction after counsel review, and make retention configurable
- X. Other (please specify)

[Answer]: B — 2y / 6m

## Q6. [Compliance specialist] Do we promise an availability level at launch?

BRD-001 and BRD-012 both target 99.9% monthly for paid plans but leave the commercial promise
open (BRD-012 OQ-004).

- A. Target 99.9% monthly for paid plans, published as a target with no service credits
- B. Publish 99.9% with service credits
- C. No published availability promise at launch
- X. Other (please specify)

[Answer]: A — 99.9% target, no credits

## Q7. [Product manager] What is the Must boundary for this release?

The scope document commits 186.0 engineer-weeks of Must work and excludes 14.5 engineer-weeks
of Should fast-follows (xpub/watch-only, PDF briefs, benchmark attribution, three extra
locales, PWA installability, and eleven smaller slices).

- A. The committed Must set is the baseline; the Should fast-follows are out of this release
- B. Include the Should fast-follows in this release
- C. Trim further to the 9-month cut list
- X. Other (please specify)

[Answer]: A — Committed Must only

## Q8. [Product manager] "EN + VN" — which markets, and which languages?

Your answer names English and Vietnamese. The scope document had EU (MiCA) and UK. Please pin
the launch markets and the UI languages, since jurisdiction gating depends on it.

- A. Vietnam plus the EU/UK markets, UI in English and Vietnamese
- B. Vietnam only, UI in English and Vietnamese
- C. Vietnam plus EU/UK, UI in English at launch and Vietnamese later
- X. Other (please specify)

[Answer]: A — Vietnam + EU/UK, EN+VN

## Q9. [Product manager] Open source and free — does billing leave the scope?

BRD-001 and BRD-016 define plans, entitlements, quotas, checkout, and dunning. "Open source
(free)" changes that.

- A. Remove billing, plans, entitlements, and paid tiers from this release; everything is free
- B. Keep server-side usage limits to protect capacity, but no billing or plans
- C. Keep the billing capability built but ship it disabled
- X. Other (please specify)

[Answer]: A — Remove billing entirely


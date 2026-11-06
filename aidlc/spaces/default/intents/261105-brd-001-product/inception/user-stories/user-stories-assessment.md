# User Stories — Assessment

**Decision:** Execute

## Rationale

Crypto Pocket Butler is a user-facing product with multiple distinct personas, complex
business logic (valuation, cost basis, P&L, drift), and a broad set of user journeys from
onboarding to reporting. Requirements alone name the system's obligations but do not express
who does what and why, which is what the design, delivery, and test stages need next.

## Factors considered

- **Project type:** greenfield consumer product with a responsive SPA and PWA.
- **User-facing scope:** every FR group in `requirements.md` is a user-visible or user-triggered
  capability; none is internal tooling.
- **Personas:** BRD-001 defines a primary and a secondary MVP persona, plus a post-MVP
  family-office persona whose features must remain designable.
- **Complexity signals:** multi-venue aggregation, multi-chain data, cost basis and P&L,
  order intents, localized UI, and WCAG 2.2 AA.
- **Cross-team work:** backend services, gateway, SPA, platform, and observability.

## Where stories add the most value

1. **First value:** onboarding a wallet or exchange account to a first valuation, which carries
   the activation target.
2. **Trust:** valuation, freshness, and unpriced-asset behaviour, where the user must be able to
   judge whether the number is trustworthy.
3. **Action:** targets, drift, and order intents, where a wrong projection would mislead.
4. **History and reporting:** P&L coverage, snapshots, and the scheduled brief.

## Alternative coverage if skipped

Not applicable — the stage executes.

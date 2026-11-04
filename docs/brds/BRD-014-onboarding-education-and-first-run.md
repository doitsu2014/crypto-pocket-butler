# BRD-014 — Onboarding, Education & First-Run Experience

Draft v1.0 · Product/Growth · 2026-11-04. Global retail users do not arrive knowing what a read-only API key, a chain, or drift is. Activation is the product: a user must reach a trustworthy first portfolio view quickly, understand what they are seeing, and know that the product cannot move their money.

**In scope:** sign-up and first sign-in · language, base currency, and timezone · age and terms acceptance · first-run wizard · demo portfolio · on-chain address path · exchange read-only key path · CSV import path · WalletConnect path · education and tooltips · empty states · progress persistence · activation events · lifecycle email · support handoff.
**Out of scope:** KYC/AML (not required for read-only aggregation; revisit if regulated services are added) · investment advice · tax onboarding · native app-store onboarding (covered by BRD-001/012) · paid acquisition.

## Personas and jobs

| Persona | Job to be done | Primary path |
|---------|----------------|--------------|
| Beginner | "Show me what I own and whether I am up or down." | Demo → address → CSV |
| Mainstream retail | "Bring my exchanges and wallets into one view without giving up custody." | Address / WalletConnect → exchange key |
| Active investor | "Compare allocation, drift, and performance across portfolios." | Exchange key + CSV |

## Golden paths

| Path | Target time to first value | Requirements | Friction |
|------|----------------------------|--------------|----------|
| Demo portfolio | < 30 seconds | No account; sample data clearly labelled | None |
| Public address | < 2 minutes | Address + at least one enabled chain | Low |
| WalletConnect | < 2 minutes | Supported wallet; read-only session | Low |
| CSV import | < 5 minutes | Template + column mapping | Medium |
| Exchange read-only key | < 10 minutes | Guided key creation; permission verification | High |

The address path is the default for retail. The exchange path must never be presented as the only way in.

## First-run checklist

1. Choose language, base currency, and timezone.
2. Accept terms and risk disclosure; confirm age eligibility.
3. Choose a path: demo, address, WalletConnect, CSV, or exchange.
4. Add the first source; show sync progress and expected duration.
5. Create the first portfolio (name optional; a sensible default is offered).
6. Optionally set a target allocation from a template; skip is allowed.
7. Show the first valuation with freshness, coverage, and unpriced/unmapped labels.
8. Offer notifications and the first brief; both are opt-in.
9. Land on the dashboard with a short, dismissible tour.

Progress is saved; a user can leave and resume without losing the first source.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ONB-01 | Sign-up and sign-in use Supabase Auth (BRD-003); no password is handled by the application. | Must |
| FR-ONB-02 | Detect language, base currency, and timezone from the browser; let the user change them before the first source. | Must |
| FR-ONB-03 | Require terms, risk disclosure, and age confirmation before the first source. | Must |
| FR-ONB-04 | Offer a demo portfolio that uses isolated sample data and is clearly labelled. | Must |
| FR-ONB-05 | Guide the user through adding an on-chain address with chain selection and validation. | Must |
| FR-ONB-06 | Guide the user through WalletConnect or watch-only import where supported. | Should |
| FR-ONB-07 | Guide the user through CSV import with a downloadable template and mapping preview. | Must |
| FR-ONB-08 | Guide the user through exchange read-only key creation, including a per-exchange checklist and permission verification. | Must |
| FR-ONB-09 | Explain read-only access, custody, and what the product cannot do before credentials are entered. | Must |
| FR-ONB-10 | Show sync progress, partial results, and per-source errors with a recovery action. | Must |
| FR-ONB-11 | Create a default portfolio and offer target templates (for example, a simple three-asset template) without generating orders. | Should |
| FR-ONB-12 | Explain valuation freshness, unpriced assets, and unmapped assets in plain language at the point of display. | Must |
| FR-ONB-13 | Provide contextual tooltips and a glossary for allocation, drift, guardrail, cost basis, and P&L. | Should |
| FR-ONB-14 | Offer notification and brief opt-in after first value, not before. | Should |
| FR-ONB-15 | Persist onboarding progress; allow skip and resume. | Must |
| FR-ONB-16 | Record activation events: sign-up, first source, first sync, first portfolio, first valuation, first target, first notification opt-in. | Must |
| FR-ONB-17 | Send lifecycle email only with consent; include unsubscribe and localization. | Should |
| FR-ONB-18 | Hand off to support with an in-app contact action and context (no secrets). | Should |
| FR-ONB-19 | Support keyboard, screen reader, and mobile layouts for every onboarding step. | Must |
| FR-ONB-20 | Localize onboarding copy and error messages for launch locales. | Must |
| FR-ONB-21 | Allow the user to delete the demo portfolio and its sample data. | Must |
| FR-ONB-22 | Never request private keys, seed phrases, or withdrawal-enabled exchange permissions. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-ONB-01 | Demo data is isolated, clearly labelled, and never mixed with real accounts. |
| BR-ONB-02 | No dark patterns: skip is always available, and opt-ins are unchecked by default. |
| BR-ONB-03 | The product explains custody and read-only access before any credential entry. |
| BR-ONB-04 | Onboarding never asks for a private key or seed phrase. |
| BR-ONB-05 | Users can reach first value without creating an exchange API key. |
| BR-ONB-06 | Every error has a next action; no dead ends. |
| BR-ONB-07 | Marketing email requires consent and honours unsubscribe immediately. |
| BR-ONB-08 | Onboarding data is limited to what activation needs and follows BRD-017. |
| BR-ONB-09 | Localized onboarding does not change the underlying business rules. |
| BR-ONB-10 | Activation is measured from first sign-in, not from marketing attribution. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ONB-01 | A new user can open the demo portfolio without adding an account. |
| AC-ONB-02 | A user adding a valid address reaches a first valuation in under two minutes on a supported chain. |
| AC-ONB-03 | A user entering a withdrawal-enabled exchange key is rejected with a clear explanation and a link to create a read-only key. |
| AC-ONB-04 | A failed source sync shows the error and a retry action without losing earlier steps. |
| AC-ONB-05 | A user can skip the target allocation and still reach the dashboard. |
| AC-ONB-06 | Notification and brief opt-ins are off until the user enables them. |
| AC-ONB-07 | Activation events are recorded for the first source, first sync, first portfolio, and first valuation. |
| AC-ONB-08 | The demo portfolio can be deleted without affecting real data. |
| AC-ONB-09 | Onboarding completes with keyboard and screen reader at mobile width. |
| AC-ONB-10 | No onboarding response or log contains a credential or token. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Resolved by FR-ONB-04: the demo portfolio is required at launch. | Product |
| OQ-002 | Which wallets and chains are supported by WalletConnect at launch? | Product/Engineering |
| OQ-003 | Is age verification self-declaration sufficient in launch markets? | Compliance |
| OQ-004 | Which target templates are safe to offer without crossing into advice? | Product/Legal |
| OQ-005 | Which lifecycle emails are transactional versus marketing? | Legal/Product |

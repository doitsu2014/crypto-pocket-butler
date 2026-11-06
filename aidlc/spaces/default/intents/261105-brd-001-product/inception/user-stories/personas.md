# Personas — Crypto Pocket Butler

## P1 — Mai, the mainstream retail investor (primary)

- **Role:** salaried professional who buys crypto through a couple of apps and holds a wallet or two.
- **Accounts:** 1–10 accounts across exchanges and wallets.
- **Goals:**
  1. See what she owns and what it is worth without opening five apps.
  2. Know whether she is up or down, without building a spreadsheet.
  3. Understand what to look at next without crypto jargon.
- **Pain points:** values are spread across venues; P&L is guessed; she does not know whether a number is current; exchange API keys feel risky.
- **Tech comfort:** medium — uses a phone first, reads errors literally, will not debug a key permission.
- **Frequency:** weekly, more often when prices move.
- **Trust needs:** a visible price timestamp, a clear "unpriced" state, and plain-language errors.

## P2 — Tuan, the crypto-native active investor (secondary)

- **Role:** experienced investor with several wallets across chains and accounts on more than one exchange.
- **Accounts:** 3–15 wallets, multi-chain, comfortable with API keys.
- **Goals:**
  1. Consolidate every venue into one view.
  2. Compare actual allocation against a target and see drift.
  3. Keep a reliable transaction journal and P&L record.
- **Pain points:** manual reconciliation across chains; inconsistent token decimals; unclear cost basis after partial sells.
- **Tech comfort:** high — expects precision, decimal correctness, and CSV import.
- **Frequency:** daily.
- **Trust needs:** exact quantities, no guessed prices, reproducible snapshots, and an auditable correction path.

## Persona relationships and priority

Mai is the primary persona and the default for onboarding, valuation, drift, and reporting
stories. Tuan is the secondary persona and the default for multi-account, ledger, cost-basis,
and import stories. Both share the trust requirements — freshness, unpriced states, and
plain errors — so those appear in the primary persona's stories.

## Post-MVP personas (documented, no stories in this release)

- **Family office / advisor:** multi-user, client reporting, permissions. BRD-001 marks this
  post-MVP; no stories are written, and MVP decisions must not block it later.

## Internal actors (not customer personas)

- **Service operator:** administers the asset registry, runs scheduled jobs, and uses
  time-bound support access. These stories exist so the operational requirements are covered,
  but the operator is not a customer persona and has no customer-facing journey.

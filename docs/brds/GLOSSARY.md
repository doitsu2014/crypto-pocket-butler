# Business Glossary

Shared terminology for the Crypto Pocket Butler business requirements. Where a term has a precise operational meaning, the defining BRD is referenced.

| Term | Definition | Reference |
|------|------------|-----------|
| **Account** | A read-only connection to an external source of holdings: either an exchange account with API credentials or a wallet account identified by a public address. | BRD-004 |
| **Allocation** | The breakdown of a portfolio's value by asset, expressed as USD value and percentage weight. | BRD-007 |
| **Analyst** | Post-MVP role with read-only access to analytics and shared portfolios. | BRD-002 |
| **Asset** | A canonical, tradable unit in the product's catalogue (for example, BTC, ETH, USDC), identified by a unique symbol and resolved from chain-level tokens. | BRD-005 |
| **Asset contract** | The chain-specific contract or mint address that represents an asset on a particular chain. | BRD-005 |
| **Base currency** | The currency in which portfolio value is reported. The MVP uses USD. | BRD-001 |
| **Brief** | A scheduled daily or weekly portfolio report, composed deterministically in-app and readable in the SPA. | BRD-009 |
| **Chain** | A blockchain network. The product supports EVM chains and Solana. | BRD-010 |
| **Decimals** | The precision of a token on-chain; used to convert raw integer balances into human-readable quantities. | BRD-005 |
| **Default portfolio** | The portfolio a user designates as primary. Each user with at least one portfolio has exactly one default. | BRD-006 |
| **Drift** | The difference between an asset's actual weight and its target weight, in percentage points. | BRD-007 |
| **EOD snapshot** | An end-of-day point-in-time record of a portfolio's holdings and value. | BRD-007 |
| **Exchange account** | An account connected through read-only API credentials to a centralized exchange (OKX in the MVP). | BRD-004 |
| **Frozen** | A quantity held on an exchange that is not currently available for trading or withdrawal. | BRD-004 |
| **Guardrails** | User-configured constraints that recommendations must respect, such as maximum deviation, minimum trade size, and allowed assets. | BRD-006 |
| **Holding** | A quantity of an asset held in an account. | BRD-004 |
| **Manual snapshot** | A snapshot created on user request at a point in time. | BRD-007 |
| **Portfolio** | A user's strategic grouping of accounts with an optional target allocation and guardrails; the unit of valuation, drift, snapshots, and recommendations. | BRD-006 |
| **Price observation** | A recorded market price for an asset at a timestamp, including its source. | BRD-005 |
| **Recommendation** | A human-reviewed rebalancing proposal containing rationale, risk level, and suggested trades. Never executed automatically in the MVP. | BRD-008 |
| **Snapshot** | An immutable point-in-time record of a portfolio's holdings, valuation, and weights. | BRD-007 |
| **Stablecoin** | A crypto asset designed to maintain a stable value, typically pegged to a fiat currency. | BRD-005 |
| **System job** | A scheduled process that refreshes data or creates snapshots without an interactive user session. | BRD-011 |
| **Target allocation** | The user-defined intended percentage weight for each asset in a portfolio; must total 100% when defined. | BRD-006 |
| **Token registry** | The maintained list of EVM and Solana tokens, contracts/mints, and decimals used during account sync. | BRD-010 |
| **Unmapped asset** | A token found in an account that cannot be resolved to a canonical asset; excluded from valuation. | BRD-005 |
| **Unpriced asset** | An asset with a quantity but no usable price; excluded from the valued total and reported separately. | BRD-007 |
| **Valuation** | The calculation of an asset's or portfolio's value in USD using quantity and price. | BRD-007 |
| **Viewer** | Post-MVP role with limited read-only access to specific shared portfolios. | BRD-002 |
| **Wallet account** | An account connected by public address only, with one or more enabled chains. No private key material is accepted. | BRD-004 |
| **Weight** | An asset's USD value divided by the portfolio's total USD value, expressed as a percentage. | BRD-007 |

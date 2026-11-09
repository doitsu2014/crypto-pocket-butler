# Refined Mockups — Crypto Pocket Butler

Mid-fidelity structural wireframes for the core journey, designed directly from
`user-stories/stories.md` and `requirements-analysis/requirements.md` (the classic scope has no
rough-mockups stage, so there are no wireframes or user flows to refine — nothing is invented
from a missing artifact).

**Design system:** Parchment Ledger (`design-system/DESIGN.md`, `design-system/tokens.css`).
**Navigation:** app shell — 64px header, 240px sidebar on desktop, collapsing to top navigation
on tablet and phone.
**Breakpoints:** desktop ≥1024px, tablet 640–1023px, phone <640px.
**Accessibility:** WCAG 2.2 AA on core flows.

Every screen is specified in five states: empty, loading, success, error, partial. A wireframe
shows the success state; the state table says what the other four look like.

---

## S1 — Onboarding to first value

Covers US1.1, US1.2, US1.3. Purpose: get a new user to a first valuation with the fewest steps.

```
+--------------------------------------------------------------------------+
| [PB] Crypto Pocket Butler                         [Step 2 of 3]  [Exit]  |
+--------------------------------------------------------------------------+
|                                                                          |
|   Connect a wallet                                                       |
|   Add a public address to see what you own. No keys, no signing.         |
|                                                                          |
|   +------------------------------------------------------------------+   |
|   | Network                                                          |   |
|   | [ Ethereum-compatible  v ]                                       |   |
|   +------------------------------------------------------------------+   |
|   +------------------------------------------------------------------+   |
|   | Wallet address                                                   |   |
|   | 0x......................................................         |   |
|   +------------------------------------------------------------------+   |
|   We read public balances only. You can remove this at any time.         |
|                                                                          |
|   [ Add and value my wallet ]        [ I use an exchange instead ]       |
|                                                                          |
|   Progress: 1 Account  >  2 Review  >  3 First valuation                 |
+--------------------------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | First-run: the form above, no prior data, "Add and value my wallet" as the only primary action. |
| Loading | The button becomes "Valuing…" with an inline spinner; the form stays editable; a timeout message appears after 5s. |
| Success | Step 3: portfolio summary with total, as-of time, and a link to the dashboard. |
| Error | Inline below the field: "This address is not on a supported network. Supported: Ethereum-compatible, BNB Smart Chain, Solana." The address is kept. |
| Partial | Address accepted but no balances: "No assets found on this address yet. Add another account or continue." |

Annotations: address field labelled above, 14px/500; focus ring is Focus Blue; the exit link is
always available (user control); no step is timed.

---

## S2 — Connect an exchange account

Covers US1.3, US2.3. Purpose: add Binance or OKX with read-only keys and verify permissions.

```
+--------------------------------------------------------------------------+
| [PB] Crypto Pocket Butler                                    [Cancel]    |
+--------------------------------------------------------------------------+
|   Connect an exchange                                                    |
|   Use a read-only API key. We reject any key that can withdraw or trade. |
|                                                                          |
|   Exchange      [ Binance  v ]                                           |
|   API key       [ ................................ ]                     |
|   API secret    [ ................................ ]                     |
|   Label         [ Main account                  ]                        |
|                                                                          |
|   [x] I confirm this key has no withdrawal or trading permission.        |
|                                                                          |
|   [ Check permissions and connect ]                                      |
+--------------------------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | Blank form, exchange defaulted to Binance, label pre-filled from the exchange name. |
| Loading | "Checking permissions…" with an indeterminate bar; fields disabled. |
| Success | Account card in the portfolio with last-synced time and status "Synced". |
| Error | "These keys have withdrawal permission, so we cannot store them. Create a read-only key and try again." Nothing is saved. |
| Partial | Keys valid but some balances unavailable: the account appears with "3 of 5 balances synced" and a retry action. |

---

## S3 — Dashboard and portfolio

Covers US3.1, US3.2, US3.3, US3.4, US3.5, US4.4, US5.2. Purpose: the primary read.

```
+--------------------------------------------------------------------------+
| [PB]  Portfolio v                        [Search]        [Mai] [Bell]    |
+----------------+---------------------------------------------------------+
| Dashboard      | Total value            As of 14:32 UTC   [Live]         |
| Portfolios  >  | $124,580.22                                                |
| Accounts       | +$8,420.10 (+7.2%) this month  ^ up                       |
| Briefs         |                                                           |
| Settings       | +-----------------------------------------------------+ |
|                | | Asset   | Quantity    | Value      | Drift   | Status  | |
|                | |---------+-------------+------------+---------+---------| |
|                | | BTC     | 1.2480      | $82,410.00 | +10.0pp | Live    | |
|                | | ETH     | 12.4000     | $31,220.10 | -2.4pp  | Live    | |
|                | | SOL     | 210.0000    | $10,950.12 | —       | Stale   | |
|                | | —       | —           | —          | —       | Unpriced| |
|                | +-----------------------------------------------------+ |
|                | Unpriced: 2 assets, excluded from total. [Review]     |
+----------------+---------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | Centred empty state: serif "No portfolio yet", one sentence, primary "Add your first account". |
| Loading | Skeleton tiles and table rows in the content area; sidebar and header stay live. |
| Success | As above. |
| Error | Warm-sand callout: "We could not load your portfolio. Your last known values are shown and marked stale." Retry button. |
| Partial | Some accounts failed: a callout naming the accounts, partial totals labelled "Partial — 2 of 4 accounts", and the failed accounts listed. |

Annotations: every figure mono, tabular, right-aligned; every value carries an as-of time;
status is a badge with dot + word; drift in percentage points ("pp"); missing value is an em
dash.

---

## S4 — Asset detail

Covers US3.3, US3.4, US4.2, US4.3, US4.4. Purpose: explain one number.

```
+--------------------------------------------------------------------------+
| < Back to portfolio                                                      |
|                                                                          |
|   Bitcoin (BTC)                                     [Live] As of 14:32  |
|   $82,410.00        1.2480 BTC                                           |
|                                                                          |
|   +---------------------------+  +-----------------------------------+   |
|   | Cost basis     $61,200.00 |  | Unrealized     +$21,210.00 (+34.7%)|   |
|   | Net invested   $61,200.00 |  | Realized       +$1,040.00          |   |
|   +---------------------------+  +-----------------------------------+   |
|                                                                          |
|   Price history                                       [1M] [3M] [1Y]     |
|   +------------------------------------------------------------------+   |
|   |            /\\    /\\                                              |   |
|   |       /\\  /  \\  /  \\                                             |   |
|   |  /\\  /  \\/    \\/    \\                                            |   |
|   +------------------------------------------------------------------+   |
|   Source: Binance, CoinGecko   Freshness: 42s ago                       |
+--------------------------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | "No transactions for this asset yet" with an import action. |
| Loading | Skeleton tiles and chart placeholder. |
| Success | As above. |
| Error | "Price unavailable from all sources. Showing your last known price, marked stale." |
| Partial | P&L coverage: "Cost basis covers 80% of holdings; 20% has no acquisition history." |

---

## S5 — Drift and order intent

Covers US5.1, US5.2, US5.3, US5.4, US5.5, US5.6. Purpose: turn the view into an action.

```
+--------------------------------------------------------------------------+
|   Target allocation                         [Edit targets]               |
|                                                                          |
|   BTC  target 50%  actual 60%  drift +10.0pp  [Outside guardrail]        |
|   +------------------------------------------------------------------+   |
|   |########################          |                              |   |
|   +------------------------------------------------------------------+   |
|   ETH  target 35%  actual 30%  drift -5.0pp   [Within guardrail]         |
|                                                                          |
|   [ Create a buy intent ]                                                |
+--------------------------------------------------------------------------+
|   Buy BTC                                                                |
|   Amount      [ 0.0500        ] BTC      = $3,301.00                     |
|   From        [ USD balance   v ]                                        |
|                                                                          |
|   Projected after this trade                                             |
|   BTC  60% -> 62.4%   drift +12.4pp                                      |
|   ETH  30% -> 27.9%   drift -7.1pp                                       |
|                                                                          |
|   This records an intent. It does not place an order.                    |
|   [ Record intent ]      [ Cancel ]                                      |
+--------------------------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | No targets: "Set a target to see drift" with a primary "Set targets". |
| Loading | Projection area shows "Calculating…"; the record button is disabled. |
| Success | Intent recorded, allocation updates immediately; a toast with an undo window. |
| Error | "This intent is larger than your available USD balance by $120.00." Record is disabled. |
| Partial | Targets do not sum to 100%: a warning line "Targets total 95% — 5% unallocated" before saving. |

---

## S6 — Scheduled brief

Covers US6.1, US6.2, US6.3, US6.4. Purpose: remove manual reporting.

```
+--------------------------------------------------------------------------+
|   Briefs                                              [Preferences]      |
|                                                                          |
|   Weekly brief — 3 Nov 2026                            [Delivered]       |
|   +------------------------------------------------------------------+   |
|   | Total value $124,580.22   ^ +7.2% this week                      |   |
|   | Largest move: BTC +4.1%   Drift: BTC +10.0pp outside guardrail   |   |
|   | Unpriced: 2 assets excluded                                      |   |
|   | [ Open in app ]   [ Also sent by email ]                         |   |
|   +------------------------------------------------------------------+   |
+--------------------------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | "Your first brief arrives after your first full week" with a sample labelled "Sample data — illustrative only." |
| Loading | Skeleton brief card. |
| Success | As above. |
| Error | "This brief could not be generated. We will retry; you can also refresh now." |
| Partial | Email failed but in-app exists: a badge "Email failed — retry" beside the brief. |

---

## S7 — Settings and privacy

Covers US7.1–US7.6, US3.5. Purpose: control the account and its data.

```
+--------------------------------------------------------------------------+
|   Settings                                                               |
|   [ Profile ] [ Security ] [ Notifications ] [ Data & privacy ]          |
+--------------------------------------------------------------------------+
|   Security                                                               |
|   Two-factor authentication                     [ Enabled ]  [Manage]    |
|   Devices and sessions                                                   |
|     MacBook Pro - Ho Chi Minh City - 2 min ago             [This device]  |
|     iPhone 15 - Ho Chi Minh City - yesterday               [Revoke]      |
|                                                                          |
|   Data & privacy                                                         |
|   [ Export my data ]     [ Delete my account ]                           |
|   Analytics consent      [ On ]                                          |
|   Marketing consent      [ Off ]                                         |
+--------------------------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | No sessions beyond the current one: "This is your only active session." |
| Loading | Section skeletons; destructive actions disabled while loading. |
| Success | As above. |
| Error | "We could not load your sessions. Try again." with a retry. |
| Partial | Export in progress: "Preparing your export — we will notify you when it is ready." |

---

## S8 — Support and status

Covers US8.1. Purpose: help and incident visibility.

```
+--------------------------------------------------------------------------+
|   Help                                                                   |
|   [ Search help articles                                  ]              |
|                                                                          |
|   Popular: Connect an exchange · Fix a stale balance · Export data       |
|                                                                          |
|   +------------------------------------------------------------------+   |
|   | Service status                          All systems operational  |   |
|   | [ View status page ]                                             |   |
|   +------------------------------------------------------------------+   |
|                                                                          |
|   Still need help?  [ Contact support ]                                  |
+--------------------------------------------------------------------------+
```

| State | What the user sees |
|---|---|
| Empty | No search results: "No articles matched. Try a different word or contact support." |
| Loading | Search skeleton rows. |
| Success | As above. |
| Error | "Help is temporarily unavailable. The status page still works." |
| Partial | Degraded service: the status card shows the affected component and the last update time. |

---

## Screen-to-story coverage

| Screen | Stories |
|---|---|
| S1 Onboarding | US1.1, US1.2, US1.3 |
| S2 Connect an account | US1.3, US2.3 |
| S3 Dashboard and portfolio | US3.1–US3.5, US4.4, US5.2 |
| S4 Asset detail | US3.3, US3.4, US4.2, US4.3, US4.4 |
| S5 Drift and order intent | US5.1–US5.6 |
| S6 Brief | US6.1–US6.4 |
| S7 Settings and privacy | US3.5, US7.1–US7.6 |
| S8 Support and status | US8.1 |

Screens not mocked here are covered by the component specifications in `interaction-spec.md`
and the story criteria: account management (US2.1–US2.3) reuses the connect wizard and the
account card; transaction ledger and corrections (US4.1, US4.5) reuse the holdings table
pattern; operator screens (US9.1–US9.3) are internal and follow the same shell without the
customer chrome.

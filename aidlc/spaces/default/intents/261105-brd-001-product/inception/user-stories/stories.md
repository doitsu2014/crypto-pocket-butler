# User Stories — Crypto Pocket Butler

Stories are grouped by user journey. Each carries a MoSCoW priority, the requirements it
traces to, and 3–6 Given/When/Then acceptance criteria. IDs are stable: `US{group}.{seq}` and
`AC{group}.{seq}.{criterion}`.

Priority: **Must Have** is the committed scope for this release. **Should Have** marks the
fast-follow set, which is not built in this release.

---

## US1 — Onboarding to first value

### US1.1 Create an account and verify email — Must Have

As Mai, I want to create an account and verify my email, so that my portfolio is private and secure.

Traces: FR1.1, FR1.5.

- **AC1.1.1** Given a new visitor, when they sign up with a valid email and password, then a verification email is sent and no portfolio data is reachable before verification.
- **AC1.1.2** Given an unverified account, when the user opens the dashboard, then they are routed to verification with a resend option.
- **AC1.1.3** Given an invalid or already-registered email, when they submit, then a plain-language error names the problem and the form keeps everything they typed.

### US1.2 Reach a first valuation from a wallet address — Must Have

As Mai, I want to add a public wallet address and see its value, so that I reach first value in under two minutes.

Traces: FR2.2, FR12.1, NFR6.

- **AC1.2.1** Given a supported address, when it is submitted, then a valuation appears within two minutes of submission.
- **AC1.2.2** Given an unsupported or malformed address, when submitted, then an error names the supported networks and keeps the address in the field.
- **AC1.2.3** Given an address with no balances, when added, then an empty state says no assets were found and offers the next action.

### US1.3 Add an exchange account with read-only keys — Must Have

As Tuan, I want to add an exchange account with read-only API keys, so that my exchange holdings join the portfolio without withdrawal risk.

Traces: FR2.1, FR2.6.

- **AC1.3.1** Given read-only Binance or OKX keys, when added, then the account syncs and its balances appear with a last-synced time.
- **AC1.3.2** Given keys with withdrawal or trading permission, when added, then they are rejected with the reason and nothing is stored.
- **AC1.3.3** Given a key that fails verification, when added, then the failure is shown and the user can retry without re-entering the label.

### US1.4 Import transaction history from CSV — Must Have

As Tuan, I want to import transactions from a CSV, so that my history is complete without manual entry.

Traces: FR2.3, FR5.1.

- **AC1.4.1** Given a valid CSV, when imported, then valid rows are stored and a summary shows imported and skipped counts.
- **AC1.4.2** Given duplicate rows, when imported, then duplicates are skipped and counted.
- **AC1.4.3** Given malformed rows, when imported, then every valid row is still imported and each bad row is reported with its line number.

---

## US2 — Connect and manage accounts

### US2.1 Add wallets across the launch networks — Must Have

As Tuan, I want to add wallets on EVM chains, BNB Smart Chain, and Solana, so that all my on-chain holdings are in one place.

Traces: FR2.2.

- **AC2.1.1** Given an Ethereum-compatible address, when added, then its token balances are shown and priced where a price exists.
- **AC2.1.2** Given a BNB Smart Chain address, when added, then BEP20 token balances are shown.
- **AC2.1.3** Given a Solana address, when added, then its SPL balances are shown.
- **AC2.1.4** Given an address on a network outside launch coverage, when added, then it is rejected with the supported list.

### US2.2 Connect read-only with WalletConnect — Must Have

As Tuan, I want to connect with WalletConnect read-only, so that I can view a wallet without pasting an address.

Traces: FR2.2.

- **AC2.2.1** Given a WalletConnect session, when it is approved, then the wallet's balances appear and the session is read-only.
- **AC2.2.2** Given a rejected or expired session, when it ends, then no account is added and the user can retry.
- **AC2.2.3** Given a connected session, when the user opens it, then no signing or transfer action is offered.

### US2.3 See sync status and re-sync an account — Must Have

As Mai, I want to see each account's sync status and re-sync it, so that I know the data is current.

Traces: FR2.5.

- **AC2.3.1** Given an account, when the portfolio is viewed, then its last-synced time and status are shown.
- **AC2.3.2** Given a stale account, when the user triggers a re-sync, then the status shows in-progress and then a new time or a visible failure.
- **AC2.3.3** Given a failed re-sync, when it fails, then the reason is shown and the previous balances are kept and labelled.

---

## US3 — See the portfolio

### US3.1 One consolidated portfolio view — Must Have

As Mai, I want one portfolio view in my base currency, so that I know what I own and what it is worth.

Traces: FR4.1, FR3.3, FR3.4.

- **AC3.1.1** Given accounts with holdings, when the portfolio opens, then one total and a per-asset breakdown are shown in the base currency.
- **AC3.1.2** Given a foreign-currency holding, when valued, then the FX rate and its timestamp are shown.
- **AC3.1.3** Given the portfolio, when it loads, then each asset shows its price timestamp.

### US3.2 Keep more than one portfolio — Must Have

As Tuan, I want more than one portfolio, so that I can separate strategies.

Traces: FR2.4, FR4.1.

- **AC3.2.1** Given two portfolios, when accounts are assigned, then each portfolio values only its own accounts.
- **AC3.2.2** Given an account in one portfolio, when the user moves it, then both totals update.
- **AC3.2.3** Given no portfolios, when the user creates the first one, then it is created and opened.

### US3.3 Trust the price freshness and source — Must Have

As Mai, I want to see price freshness and source, so that I can judge whether the valuation is trustworthy.

Traces: FR3.4, FR3.2.

- **AC3.3.1** Given a price within the freshness bound, when shown, then its timestamp and source are visible.
- **AC3.3.2** Given a price older than the bound, when shown, then the asset is marked stale with its timestamp.
- **AC3.3.3** Given the primary price source failing, when a price is served, then the fallback source is used and recorded.

### US3.4 See unpriced assets honestly — Must Have

As Mai, I want unpriced assets clearly marked, so that totals are not misleading.

Traces: FR3.5.

- **AC3.4.1** Given an asset with no price source, when the portfolio is valued, then it is excluded from the priced total and labelled unpriced.
- **AC3.4.2** Given a portfolio containing an unpriced asset, when the total is shown, then the excluded value is stated, not silently dropped.
- **AC3.4.3** Given an unpriced asset, when a price source later becomes available, then the next refresh prices it and clears the label.

### US3.5 Change base currency, language, and timezone — Must Have

As Mai, I want to change base currency, language, and timezone, so that values match how I think.

Traces: FR4.5, FR9.1.

- **AC3.5.1** Given a user changing the base currency, when the portfolio reloads, then all values use the new currency and its FX rates.
- **AC3.5.2** Given Vietnamese selected, when core flows load, then they are shown in Vietnamese.
- **AC3.5.3** Given a changed timezone, when timestamps are shown, then they render in that timezone.

---

## US4 — Understand performance

### US4.1 A single transaction ledger — Must Have

As Tuan, I want a transaction ledger, so that every movement is recorded once.

Traces: FR5.1.

- **AC4.1.1** Given the same trade imported from an exchange and a CSV, when both are processed, then one ledger entry exists and the duplicate is reported.
- **AC4.1.2** Given a transaction, when it is viewed, then its source, timestamp, asset, quantity, and price are shown.
- **AC4.1.3** Given a transaction recorded, when later data changes, then the ledger entry is unchanged.

### US4.2 Cost basis and P&L — Must Have

As Mai, I want cost basis and P&L per asset and portfolio, so that I know whether I am up or down.

Traces: FR5.2.

- **AC4.2.1** Given a buy and a partial sell, when P&L is shown, then realized and unrealized amounts reconcile to the ledger.
- **AC4.2.2** Given a portfolio with history, when P&L is requested, then net invested and total return are shown per asset and in total.
- **AC4.2.3** Given a transaction correction, when P&L recalculates, then the corrected figures reconcile to the corrected ledger.

### US4.3 Label P&L coverage — Must Have

As Mai, I want to see when P&L is unavailable, so that I do not trust a fabricated number.

Traces: FR5.3.

- **AC4.3.1** Given a portfolio with no acquisition history, when P&L is requested, then it is shown as unavailable with a coverage label.
- **AC4.3.2** Given partial history, when P&L is shown, then the covered portion and the gap are both stated.
- **AC4.3.3** Given an imported history that completes the gap, when recalculated, then the label clears.

### US4.4 Immutable snapshots and history — Must Have

As Mai, I want snapshots and their history, so that I can see how my portfolio changed.

Traces: FR4.4, NFR8.

- **AC4.4.1** Given a snapshot, when later data changes, then the snapshot's stored values are unchanged.
- **AC4.4.2** Given a snapshot, when it is reproduced from stored inputs, then the values match exactly.
- **AC4.4.3** Given a year of history, when the history view loads, then it responds within two seconds at p95.

### US4.5 Correct a transaction — Must Have

As Tuan, I want to correct a transaction, so that my ledger stays accurate.

Traces: FR5.4.

- **AC4.5.1** Given a wrong quantity, when the user corrects it, then the ledger shows the correction and the original remains recoverable.
- **AC4.5.2** Given a correction, when it is saved, then an audit record names the actor and the change.
- **AC4.5.3** Given an invalid correction, when it is submitted, then it is refused with a field-level reason.

---

## US5 — Act on drift

### US5.1 Set target allocation and guardrails — Must Have

As Tuan, I want to set target allocations and guardrails, so that drift is measured against my plan.

Traces: FR4.2.

- **AC5.1.1** Given a target of 50% BTC, when saved, then the portfolio shows that target.
- **AC5.1.2** Given targets that do not sum to 100%, when saved, then the difference is shown before saving.
- **AC5.1.3** Given a guardrail, when saved, then its threshold is shown next to the target.

### US5.2 See drift per asset — Must Have

As Mai, I want to see drift per asset, so that I know where I am off plan.

Traces: FR4.3.

- **AC5.2.1** Given a portfolio with targets, when viewed, then every asset shows actual, target, and drift.
- **AC5.2.2** Given actual 60% against a 50% target, when shown, then drift reads +10 percentage points.
- **AC5.2.3** Given no targets, when the portfolio is viewed, then drift is absent with a prompt to set targets.

### US5.3 Create an order intent with a projected allocation — Must Have

As Mai, I want to create a buy or sell intent and see the projected allocation, so that I can act without guessing.

Traces: FR6.1, FR6.2.

- **AC5.3.1** Given a portfolio, when the user creates a buy intent, then it records asset, quantity, and portfolio.
- **AC5.3.2** Given an intent, when the preview is shown, then post-trade allocation and drift are projected.
- **AC5.3.3** Given an intent larger than the available balance, when previewed, then the shortfall is shown before it is recorded.

### US5.4 Record an executed trade — Must Have

As Mai, I want to record an executed trade, so that allocation updates immediately.

Traces: FR6.4.

- **AC5.4.1** Given a recorded trade, when the portfolio reloads, then allocation and drift reflect it without waiting for the next sync.
- **AC5.4.2** Given a recorded trade, when it is saved, then a ledger entry and an audit record are created.
- **AC5.4.3** Given a trade with a missing price, when recorded, then the user must supply one and it is labelled as user-entered.

### US5.5 Cancel a pending intent — Must Have

As Mai, I want to cancel a pending intent, so that a mistake is not acted on.

Traces: FR6.3.

- **AC5.5.1** Given a pending intent, when the user cancels it, then it is marked cancelled.
- **AC5.5.2** Given any intent, when it is created or cancelled, then no external order call is made.
- **AC5.5.3** Given a cancelled intent, when the list is viewed, then it remains visible with its cancellation time.

### US5.6 Drift guardrail notification — Must Have

As Mai, I want to be alerted when drift passes a guardrail, so that I do not miss it.

Traces: FR7.4.

- **AC5.6.1** Given a guardrail of 5 points and a drift of 6, when the next evaluation runs, then one notification is produced.
- **AC5.6.2** Given the same drift on the next evaluation, when it runs, then no duplicate notification is sent.
- **AC5.6.3** Given drift back inside the guardrail, when it recrosses later, then a new notification is produced.

---

## US6 — Report and notify

### US6.1 Scheduled brief in-app — Must Have

As Mai, I want a scheduled brief in the app, so that I do not assemble reports by hand.

Traces: FR7.1.

- **AC6.1.1** Given a scheduled time, when it fires, then the brief is generated and readable in-app.
- **AC6.1.2** Given generation fails, when it fails, then the brief is explicitly marked failed with a reason.
- **AC6.1.3** Given a generated brief, when it is opened, then it states the period and the data freshness.

### US6.2 Brief by email — Must Have

As Mai, I want the brief by email, so that I can read it away from the app.

Traces: FR7.1, FR7.2.

- **AC6.2.1** Given a generated brief, when delivery is attempted, then the email is sent to the verified address.
- **AC6.2.2** Given a delivery failure, when it fails, then it is retried within bounds and the outcome is recorded.
- **AC6.2.3** Given email notifications disabled, when a brief is generated, then it stays in-app and no email is sent.

### US6.3 Notification preferences — Must Have

As Mai, I want notification preferences, so that I only get what I asked for.

Traces: FR7.2.

- **AC6.3.1** Given a disabled notification type, when its event occurs, then no notification of that type is delivered.
- **AC6.3.2** Given a changed preference, when saved, then it applies to the next event.
- **AC6.3.3** Given preferences, when they are viewed, then each type shows whether it is on for in-app and email.

### US6.4 Mandatory security and credential email — Must Have

As Mai, I want mandatory security and credential emails, so that I hear about account risk.

Traces: FR7.3.

- **AC6.4.1** Given a credential revocation event, when it occurs, then an email is sent even if general notifications are off.
- **AC6.4.2** Given a new-device sign-in, when it occurs, then a security email is sent.
- **AC6.4.3** Given a mandatory email, when it is sent, then it carries no credentials or full addresses.

---

## US7 — Account, privacy, and settings

### US7.1 Enrol in MFA — Must Have

As Mai, I want MFA, so that my account is protected.

Traces: FR1.2.

- **AC7.1.1** Given a user with an exchange account and no MFA, when they sign in, then they must enrol before portfolio data is shown.
- **AC7.1.2** Given MFA enrolment, when it completes, then recovery codes are shown once and stored by the user.
- **AC7.1.3** Given a wrong MFA code, when submitted, then it is refused and the attempt is rate-limited.

### US7.2 See and revoke sessions — Must Have

As Tuan, I want to see and revoke sessions, so that a lost device cannot keep access.

Traces: FR1.4.

- **AC7.2.1** Given two active sessions, when the user revokes one, then that session is signed out at its next request.
- **AC7.2.2** Given the session list, when it is viewed, then each session shows device, location, and last activity.
- **AC7.2.3** Given the current session, when the user tries to revoke it, then they are warned that they will be signed out.

### US7.3 Export my data — Must Have

As Mai, I want to export my data, so that I can take it with me.

Traces: FR8.1, NFR13.

- **AC7.3.1** Given an export request, when it completes, then a machine-readable file contains accounts, portfolios, transactions, and snapshots.
- **AC7.3.2** Given a large export, when it runs, then progress is shown and the file is delivered when ready.
- **AC7.3.3** Given an export, when it is produced, then it contains no credentials or tokens.

### US7.4 Delete my account — Must Have

As Mai, I want to delete my account, so that my data is removed.

Traces: FR8.2, NFR13.

- **AC7.4.1** Given a deletion request, when confirmed, then personal data is deleted or anonymised.
- **AC7.4.2** Given retained financial records, when deletion completes, then they are kept only under the stated legal basis and labelled.
- **AC7.4.3** Given a deletion in progress, when the user signs in, then the state is explained and no new data is collected.

### US7.5 Control analytics and marketing consent — Must Have

As Mai, I want to control analytics and marketing consent, so that I decide what is shared.

Traces: FR8.3, FR12.2.

- **AC7.5.1** Given a new user, when consent is not given, then no non-essential analytics event is sent.
- **AC7.5.2** Given consent withdrawn, when saved, then later events are not sent.
- **AC7.5.3** Given the consent screen, when it is shown, then no box is pre-ticked.

### US7.6 Jurisdiction and age gate — Must Have

As Mai, I want jurisdiction and age gating, so that the service is lawful where I live.

Traces: FR8.4.

- **AC7.6.1** Given a user in a blocked jurisdiction, when they reach sign-up, then access is denied before any personal data is collected.
- **AC7.6.2** Given a user in a supported jurisdiction, when they sign up, then the required disclosures are shown and acknowledged.
- **AC7.6.3** Given an age below the threshold, when declared, then access is denied with a plain explanation.

---

## US8 — Support and status

### US8.1 Help centre, support channel, and status page — Must Have

As Mai, I want a help centre, a support channel, and a status page, so that I can get help and know when something is broken.

Traces: FR11.1.

- **AC8.1.1** Given a user, when they open help, then searchable articles and a contact channel are available.
- **AC8.1.2** Given a declared incident, when it is declared, then the status page shows it.
- **AC8.1.3** Given a SEV-1 incident, when it is declared, then communication goes out within 15 minutes.

---

## US9 — Internal operations

### US9.1 Scheduled jobs with heartbeats and alerts — Must Have

As the service operator, I want scheduled refresh, sync, FX, and notification jobs with heartbeats, so that data stays fresh and failures are visible.

Traces: FR10.1, FR10.2, FR10.3, NFR17.

- **AC9.1.1** Given a schedule, when the interval elapses, then the job runs and records its outcome.
- **AC9.1.2** Given a stopped scheduler, when the heartbeat is missed, then an alert fires within five minutes.
- **AC9.1.3** Given a transient provider failure, when the call is retried, then the side effect is applied once.

### US9.2 Manage the asset registry with audit — Must Have

As the service operator, I want to manage the asset registry and see audited changes, so that supported assets stay correct.

Traces: FR3.1, FR11.3.

- **AC9.2.1** Given a new token contract, when it is added, then it resolves to one canonical asset with its decimals.
- **AC9.2.2** Given an admin registry change, when it is saved, then an audit record names actor, target, time, and outcome.
- **AC9.2.3** Given a token on two chains, when prices are combined, then the two contracts are not double counted.

### US9.3 Time-bound audited support access — Must Have

As the service operator, I want time-bound, audited support access, so that I can help without over-reaching.

Traces: FR11.2.

- **AC9.3.1** Given a support session, when it expires, then access ends.
- **AC9.3.2** Given a support session, when it starts, then a reason and a second approver are required.
- **AC9.3.3** Given any support read, when it happens, then it is recorded in the audit trail.

---

## Story map

| Journey step | Stories | MVP line |
|---|---|---|
| Onboarding | US1.1–US1.4 | Above |
| Connect accounts | US2.1–US2.3 | Above |
| See the portfolio | US3.1–US3.5 | Above |
| Understand performance | US4.1–US4.5 | Above |
| Act on drift | US5.1–US5.6 | Above |
| Report and notify | US6.1–US6.4 | Above |
| Account and privacy | US7.1–US7.6 | Above |
| Support and status | US8.1 | Above |
| Internal operations | US9.1–US9.3 | Above (operator) |

## Deferred and uncovered requirements

- **NFR1–NFR20** are formalised in the NFR Requirements stage; the stories that depend on them
  state the user-visible behaviour, and the numeric targets are traced there.
- **FR13.1 and FR13.2** (gateway API surface, deny-by-default routing, rate limits, correlation
  IDs) are architectural and are designed in Domain Design.
- **Should Have fast-follows** — xpub/watch-only, PDF briefs, benchmark attribution, extra
  locales, PWA installability, and the smaller slices listed in the scope document — are not
  written as stories in this release.

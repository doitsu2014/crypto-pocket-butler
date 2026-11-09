# Frontend Components — u2-web-app

Inputs: `functional-spec.md`, `design-system/DESIGN.md`, `design-system/tokens.css`, `web/src/styles.css`.

Router: **React Router** (`createBrowserRouter`) renders the route table; `/support` is not a route
of this app (the U13 bundle serves it).

## Shell

| Component | Props / state | Behaviour |
|---|---|---|
| `AppShell` | `children` | header, navigation, locale switch, sign-out; skips navigation on `/` and auth routes |
| `RequireSession` | `children` | state from the session store; redirects to `/` and back to the deep link after sign-in |
| `ApiProvider` | `children` | TanStack Query client; one API client; a 401 signs out and redirects to `/`; a 403 renders the refusal; a 5xx retries once; the error panel uses the problem body |
| `StatusRegion` | `message` | `aria-live="polite"` announcements for async status |
| `ErrorBoundary` | `fallback` | full-page recovery for a failed bootstrap |

## Shared components

| Component | Props | Rules |
|---|---|---|
| `Money` | `value: string \| null`, `currency` | decimal string as-is; em dash for null; tabular mono |
| `Percent`, `Points` | `value: string \| null` | same rules |
| `DataStatusBadge` | `status`, `asOf`, `label` | label + shape + colour; covers complete/partial/unavailable and line `priced`/`unpriced`/`holdings_unavailable` |
| `FreshnessBadge` | `freshness`, `observedAt` | live/delayed/stale/unpriced with timestamp |
| `CoverageNote` | `coverage`, `excludedCount`, `marksUnavailable`, `fxUnavailable`, `fxEstimated` | labels every gap; withheld figure is an em dash |
| `PriceProvenance` | `source`, `observedAt`, `fxSource`, `fxObservedAt` | provenance text and as-of |
| `AsOf` | `at` | localised instant |
| `ErrorPanel` | `problem` | title/detail + `correlationId` + retry |
| `EmptyState` | `title`, `action` | one action, never zeros |
| `ConfirmDialog` | `title`, `confirmLabel`, `onConfirm` | destructive confirmations |
| `Table` | `caption`, `columns`, `rows` | `scope`/`caption`; keyboard scroll |

## Journey components (props, state, endpoint)

| Component | Props / state | Endpoint and failures |
|---|---|---|
| `SignInForm` | local: email, password; validation email shape and non-empty | Supabase Auth; 401 detail shown |
| `SignupGateForm` | local: gate token, email, password | `POST /v1/signup-gate`, then Auth sign-up; 422 field errors |
| `VerifyEmailPanel` | query: status | `POST /v1/auth/resend-verification`; 429 shows retry-after |
| `OnboardingSteps` | local: step, consent choices, preferences | `POST /v1/consent`, `PUT /v1/preferences`; 422 per field |
| `AccountKindPicker` | `onSelect` | none |
| `ExchangeCredentialForm` | local: venue, label, key, secret, passphrase | `POST /v1/accounts`; 422 field, 409 duplicate label, verification status polled |
| `WalletAddressForm` | local: network, address | `POST /v1/accounts`; 422 lists supported networks |
| `WalletConnectButton` | state: session | C14 read-only; no signing action rendered |
| `CsvImport` | state: file, result {accepted, skipped, errors[]} | `POST /v1/imports`, poll `GET /v1/imports/{id}`; per-line errors with line numbers |
| `AccountStatusRow` | `account` | shows last-synced, `syncing/synced/failed`; re-sync keeps labelled balances on failure |
| `PortfolioList` | query `['portfolios']` | `GET`/`POST /v1/portfolios`; empty state creates the first |
| `PortfolioSummary` | `valuation` | renders total, breakdown, status, as-of |
| `AllocationTable` | `lines` | price timestamp, source, freshness, unpriced rows kept |
| `MoveAccountDialog` | local: target portfolio | `PUT`/`DELETE /v1/portfolios/{id}/accounts/{accountId}`; both totals refetch |
| `TargetEditor` | local: rows; preview | `PUT .../targets?preview=true` then save; sum/difference from 100 shown; save disabled until 100 |
| `IntentForm` | local: side, quantity; preview | `POST .../order-intents?preview=true` then create; shortfall shown; 409 conflict |
| `IntentTable` | `intents` | cancel `POST .../{id}/cancel` |
| `TradeForm` | local: side, quantity, price, source, executedAt | `POST .../trades`; 409 duplicate shows the existing trade |
| `TransactionTable` | query `['transactions', cursor]` | C1 transactions; cursor pagination; missing price labelled |
| `CorrectionForm` | local: corrected values | `POST /v1/transactions/{id}/corrections`; 409 existing correction |
| `PnlPanel` | `pnl` | `GET /v1/users/{userId}/pnl`; coverage, marks/holdings/fx flags, estimated FX |
| `SnapshotHistory` | query `['snapshots', id, cursor]` | `GET .../snapshots`; as-of, data status, verified label |
| `BriefList` | query `['briefs', period, cursor]` | `GET /v1/users/{userId}/briefs`; pending/generated/failed badges |
| `BriefView` | `brief` | sections, failureReason when failed, as-of, coverage |
| `FeedList` | query `['notifications', cursor]` | `GET /v1/users/{userId}/notifications` |
| `PreferenceMatrix` | local: matrix | `GET`/`PUT .../preferences`; mandatory email shown locked |
| `ProfileForm`, `SecurityPanel`, `MfaPanel`, `SessionList` | local forms | C1 users/sessions/MFA routes; recovery codes shown once |
| `ConsentHistory`, `ExportPanel`, `DeletionPanel` | queries | C1 consent/exports/deletions; deletion requires ConfirmDialog |
| `HelpCentre` | query `['articles']`, `['incidents']` | `GET /v1/support/articles`, `GET /v1/support/incidents?active=true` |
| `SupportRequestForm` | local: subject, body | `POST /v1/support/requests`; refuses credentials client-side |

## Design and data rules

- Tokens from `design-system/tokens.css` / `web/src/styles.css`: `--color-parchment`,
  `--color-ink`, `--color-accent`, `--font-display`, `--font-ui`, `--font-mono`. No raw values.
- Parchment canvas, ink text, accent terracotta through `--color-accent`; serif display, sans UI,
  tabular mono figures.
- One API-client module owns gateway calls; components receive data through query hooks.
- Missing values are em dashes; a status is never colour-only; every async surface has loading,
  empty, error and degraded states.
- Per-route panel map: dashboard/portfolio → summary, allocation, drift; activity → transactions,
  P&L; briefs → header, sections; settings → profile, security, sessions; privacy → consent,
  export, deletion; help → articles, incidents. A source outage renders only its panel.

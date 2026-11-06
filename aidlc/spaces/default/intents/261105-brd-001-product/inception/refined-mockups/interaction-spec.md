# Interaction Specification — Crypto Pocket Butler

Component-level specifications follow `.claude/knowledge/aidlc-design-agent/component-spec-template.md`.
Tokens are the Parchment Ledger tokens in `design-system/tokens.css`; use `var()` only.

## Global interaction rules

- **Navigation:** 64px header + 240px sidebar at ≥1024px; sidebar collapses to top navigation at
  640–1023px and a menu button below 640px. Current section is highlighted with the terracotta
  underline treatment used for tabs.
- **Forms:** labels above inputs; validate on blur, not per keystroke; errors below the field with
  a written message; submit disabled only when a required field is empty, with the reason shown.
- **Modals:** used only for confirmations and small focused input; focus trapped, Escape closes,
  focus returns to the trigger; never nested.
- **Loading:** skeleton screens in the content area; spinners only for actions; a timeout message
  after 5 seconds.
- **Success:** inline confirmation near the action, auto-dismiss after 5 seconds, never an
  immediate redirect.
- **Errors:** inline near the cause, crimson border plus icon plus written text — never colour
  alone.
- **Motion:** 150ms micro-states, 200ms general transitions, `cubic-bezier(0.2, 0, 0, 1)`;
  respect `prefers-reduced-motion`.
- **Financial figures:** mono, tabular numerals, right-aligned; two decimals for USD, up to six
  significant digits for crypto quantities; missing value is an em dash.

---

## Portfolio KPI Tile

| Field | Value |
|---|---|
| Component | Portfolio KPI tile |
| Description | Shows one headline figure with its delta and as-of time |
| Category | display |

### States

| State | Description | Trigger |
|---|---|---|
| default | Label, mono value, delta, as-of time | data loaded |
| loading | Skeleton value bar | data fetching |
| stale | Value shown with a "Stale" badge and last-known time | data older than the freshness bound |
| error | "Unavailable" with the last known value marked stale | fetch failed |
| empty | Em dash with "No data yet" | no data |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| label | string | yes | — | Overline label, 10px uppercase |
| value | string | yes | — | Formatted figure or em dash |
| delta | object | no | — | `{ sign, value, unit, direction }` |
| asOf | string | yes | — | Timestamp shown as meta |
| status | enum | no | live | live, delayed, stale, unpriced |

### Responsive Behaviour

| Breakpoint | Behaviour |
|---|---|
| phone (<640px) | Tiles stack single column |
| tablet (640–1023px) | Two tiles per row |
| desktop (>1024px) | Up to four tiles per row |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `group` with `aria-labelledby` on the label |
| Keyboard interaction | None — not interactive |
| Label / aria-label | Visible label; delta announced as "up 7.2 percent" |
| Contrast ratio | WCAG AA (4.5:1 text, 3:1 UI) |
| Screen reader | Reads label, value, delta direction, and as-of time |
| Focus management | Not focusable |

---

## Holdings Table

| Field | Value |
|---|---|
| Component | Holdings table |
| Description | Per-asset quantity, value, drift, and status |
| Category | display |

### States

| State | Description | Trigger |
|---|---|---|
| default | Rows with real values | data loaded |
| loading | Skeleton rows | data fetching |
| empty | Empty state below the table header | no assets |
| error | Callout above the table, last known values marked stale | fetch failed |
| partial | "Partial — 2 of 4 accounts" label and a failed-account list | some accounts failed |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| rows | array | yes | — | `{ asset, quantity, value, drift, status }` |
| sort | object | no | value desc | Sort column and direction |
| onSort | function | no | — | Sort handler |
| coverage | object | no | — | Priced and unpriced counts |

### Responsive Behaviour

| Breakpoint | Behaviour |
|---|---|
| phone (<640px) | Table scrolls horizontally inside its container with an edge fade; numbers never wrap |
| tablet (640–1023px) | Same, with fewer visible columns |
| desktop (>1024px) | Full columns |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | Real `<table>` with `<th scope="col">` and a `<caption>` |
| Keyboard interaction | Tab to the table; sortable headers are buttons with `aria-sort` |
| Label / aria-label | Caption names the portfolio and as-of time |
| Contrast ratio | WCAG AA |
| Screen reader | Each cell announces header context |
| Focus management | Focus ring on sortable headers |

---

## Drift Bar

| Field | Value |
|---|---|
| Component | Drift bar |
| Description | Actual against target allocation for one asset, with a guardrail state |
| Category | display |

### States

| State | Description | Trigger |
|---|---|---|
| within | Bar inside the guardrail | drift inside threshold |
| outside | Bar outside the threshold, amber | drift outside threshold |
| breached | Bar past the hard limit, crimson | drift past the limit |
| unknown | Em dash and "No target set" | no target |
| loading | Skeleton bar | data fetching |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| actual | number | yes | — | Actual allocation percentage |
| target | number | yes | — | Target percentage |
| guardrail | number | no | — | Threshold in percentage points |
| state | enum | yes | within | within, outside, breached, unknown |

### Responsive Behaviour

| Breakpoint | Behaviour |
|---|---|
| phone (<640px) | Bar full width, label above |
| tablet (640–1023px) | Bar and label side by side |
| desktop (>1024px) | As tablet |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `img` with a descriptive `aria-label`, or `meter` where supported |
| Keyboard interaction | None |
| Label / aria-label | "BTC actual 60 percent, target 50 percent, drift plus 10 percentage points, outside guardrail" |
| Contrast ratio | WCAG AA; state also carries a word |
| Screen reader | Reads the full sentence above |
| Focus management | Not focusable |

---

## Status Badge

| Field | Value |
|---|---|
| Component | Status badge |
| Description | Pill showing live, delayed, stale, unpriced, synced, or failed |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| live | Green dot + "Live" | fresh data |
| delayed | Amber dot + "Delayed" | past soft bound |
| stale | Amber dot + "Stale" | past hard bound |
| unpriced | Stone dot + "Unpriced" | no price source |
| failed | Crimson dot + "Failed" | fetch or sync failed |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| status | enum | yes | — | live, delayed, stale, unpriced, failed, synced |
| label | string | no | derived | Overrides the visible word |

### Responsive Behaviour

All breakpoints: pill inline with the value; wraps below it on phone.

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `status` for changing values |
| Keyboard interaction | None |
| Label / aria-label | The visible word is the accessible name |
| Contrast ratio | WCAG AA; dot is decorative, the word carries meaning |
| Screen reader | Announces the status word when it changes |
| Focus management | Not focusable |

---

## Order Intent Panel

| Field | Value |
|---|---|
| Component | Order intent panel |
| Description | Buy or sell intent with projected allocation before it is recorded |
| Category | input |

### States

| State | Description | Trigger |
|---|---|---|
| default | Form with amount and source | panel opened |
| loading | "Calculating…" projection, record disabled | projection running |
| valid | Projection shown, record enabled | valid amount |
| invalid | Field error, record disabled | amount exceeds balance or is malformed |
| recorded | Confirmation toast with undo | intent saved |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| side | enum | yes | buy | buy or sell |
| asset | object | yes | — | Asset being traded |
| portfolio | object | yes | — | Target portfolio |
| available | string | yes | — | Available balance in the source currency |
| onRecord | function | yes | — | Records the intent |

### Responsive Behaviour

| Breakpoint | Behaviour |
|---|---|
| phone (<640px) | Full-screen sheet |
| tablet (640–1023px) | Centred dialog |
| desktop (>1024px) | Side panel beside the drift view |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `dialog` with `aria-labelledby` |
| Keyboard interaction | Tab through fields; Escape closes with a confirmation if dirty |
| Label / aria-label | Title "Buy BTC"; the "does not place an order" note is described by `aria-describedby` |
| Contrast ratio | WCAG AA |
| Screen reader | Projection changes announced through `aria-live="polite"` |
| Focus management | Focus moves to the amount field on open; returns to the trigger on close |

---

## Connect Account Wizard

| Field | Value |
|---|---|
| Component | Connect account wizard |
| Description | Three-step flow for an exchange key or a wallet address |
| Category | input |

### States

| State | Description | Trigger |
|---|---|---|
| default | Step 1 of 3 | opened |
| loading | "Checking permissions…" with fields disabled | submit |
| success | Step 3 with first valuation | verified |
| error | Field-level error, nothing stored | rejected key or bad address |
| partial | Account added with some balances unavailable | partial sync |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| mode | enum | yes | wallet | wallet or exchange |
| onComplete | function | yes | — | Called after the account is added |

### Responsive Behaviour

| Breakpoint | Behaviour |
|---|---|
| phone (<640px) | Single column, sticky primary action |
| tablet (640–1023px) | Single column, centred |
| desktop (>1024px) | Centred 640px column |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `form` with `aria-labelledby` |
| Keyboard interaction | Tab order follows fields; Enter submits; no keyboard trap |
| Label / aria-label | Visible labels above every field; step progress is text |
| Contrast ratio | WCAG AA |
| Screen reader | Step changes announced through `aria-live="polite"` |
| Focus management | Focus moves to the first field of each step |

---

## Notification Preferences

| Field | Value |
|---|---|
| Component | Notification preferences |
| Description | Per-type in-app and email switches, with mandatory security types locked on |
| Category | input |

### States

| State | Description | Trigger |
|---|---|---|
| default | Rows with switches | loaded |
| saving | Row shows "Saving…" | toggle changed |
| saved | "Saved" inline confirmation | save succeeded |
| error | "Could not save — try again" with the switch reverted | save failed |
| locked | Mandatory type shown on and disabled with a reason | security or credential type |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| types | array | yes | — | `{ id, label, inApp, email, mandatory }` |
| onChange | function | yes | — | Saves a preference |

### Responsive Behaviour

| Breakpoint | Behaviour |
|---|---|
| phone (<640px) | One row per type, switches right-aligned |
| tablet (640–1023px) | Same |
| desktop (>1024px) | Two columns of types |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `group` per type; switches are native checkboxes styled as switches |
| Keyboard interaction | Space toggles; Tab moves between types |
| Label / aria-label | Label names the type and channel |
| Contrast ratio | WCAG AA |
| Screen reader | Locked switches announce "mandatory, cannot be turned off" |
| Focus management | Focus stays on the toggled switch |

---

## Empty State

| Field | Value |
|---|---|
| Component | Empty state |
| Description | One icon, serif title, one sentence, one primary action |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| default | Centred block | no data |
| error | Same block with a retry action instead | load failed |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `status` |
| Keyboard interaction | The primary action is a normal button |
| Label / aria-label | Title is a heading at the correct level |
| Contrast ratio | WCAG AA |
| Screen reader | Reads the title and explanation |
| Focus management | Not focusable unless it contains the primary action |

---

## Inline Error

| Field | Value |
|---|---|
| Component | Inline error |
| Description | Written message below the field with a crimson icon |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| hidden | Not rendered | field valid |
| visible | Icon + written message | validation failure |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `alert` for submit-time errors; `aria-describedby` for field errors |
| Keyboard interaction | None |
| Label / aria-label | Message text is the accessible name |
| Contrast ratio | WCAG AA |
| Screen reader | Announced when it appears |
| Focus management | Focus moves to the first invalid field on submit |

---

## Toast

| Field | Value |
|---|---|
| Component | Toast |
| Description | Brief confirmation with an optional undo |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| visible | Message with undo where relevant | action completed |
| dismissed | Hidden | 5s elapsed or dismissed |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `status` with `aria-live="polite"` |
| Keyboard interaction | Undo is reachable by keyboard; the toast does not steal focus |
| Label / aria-label | Message text |
| Contrast ratio | WCAG AA |
| Screen reader | Announced once |
| Focus management | Never moves focus |

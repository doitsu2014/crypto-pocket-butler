# Accessibility Checklist — WCAG 2.2 AA

Target: WCAG 2.2 Level AA on the core flows, zero blockers. This checklist is the design-time
commitment; the audit at the accessibility launch gate (C13) verifies it.

## Global requirements

| # | Requirement | WCAG | How it is met |
|---|---|---|---|
| A1 | Every interactive element has a visible focus indicator | 2.4.7, 2.4.11 | Focus Blue ring, minimum 2px, 3:1 contrast; never `outline: none` without replacement |
| A2 | All functionality works by keyboard alone | 2.1.1 | Tab order follows the visual layout; no keyboard traps; modals trap and release focus |
| A3 | Skip-to-content link is the first focusable element | 2.4.1 | Present on every screen in the app shell |
| A4 | Text contrast ≥4.5:1; large text and UI ≥3:1 | 1.4.3, 1.4.11 | Parchment Ledger pairs clear AA; the weakest body pair is 4.64:1 |
| A5 | Status never relies on colour alone | 1.4.1 | Every badge has a dot and a word; drift has sign, arrow, colour, and label |
| A6 | Page language is declared | 3.1.1 | `lang` set per locale (en, vi) |
| A7 | Touch targets ≥44×44px | 2.5.8 | Buttons and switches sized to 44px minimum on phone |
| A8 | Layout survives 200% text zoom and 400% page zoom | 1.4.4, 1.4.10 | Fluid grid, no fixed-height text containers |
| A9 | Motion respects `prefers-reduced-motion` | 2.3.3 | Transitions removed when the preference is set |
| A10 | Forms have visible labels and specific errors | 3.3.1, 3.3.2, 3.3.3 | Label above every input; written error below the field; suggestions included |
| A11 | Destructive actions are confirmed | 3.3.4 | Delete account and revoke session use a confirmation dialog |
| A12 | Dragging has a non-drag alternative | 2.5.7 | No drag-only interaction is specified |

## Screen-level checklist

### S1 Onboarding to first value

- [ ] Address field has a visible label and `aria-describedby` for the supported-network hint.
- [ ] Step progress is text ("Step 2 of 3"), not an image.
- [ ] Invalid address error is announced and focus returns to the field.
- [ ] The exit link is reachable by keyboard and always available.

### S2 Connect an exchange account

- [ ] Every field has a visible label; secrets are `type="password"` with a show/hide control.
- [ ] The read-only confirmation checkbox has a real label.
- [ ] A rejected key error names the permission and the next step.
- [ ] Fields are disabled during the permission check with `aria-busy`.

### S3 Dashboard and portfolio

- [ ] The holdings table is a real `<table>` with `<caption>` and `<th scope>`.
- [ ] Sortable headers are buttons with `aria-sort`.
- [ ] KPI tiles read label, value, delta direction, and as-of time in that order.
- [ ] The unpriced callout is announced as a `status`.
- [ ] Tables scroll horizontally without trapping keyboard focus.

### S4 Asset detail

- [ ] The chart has a text alternative summarising the trend and the period.
- [ ] P&L coverage is written text, not a visual-only bar.
- [ ] The stale-price error is announced and names the last known time.

### S5 Drift and order intent

- [ ] Each drift bar has an `aria-label` sentence with actual, target, drift, and guardrail state.
- [ ] The order intent dialog traps focus, closes on Escape, and returns focus to its trigger.
- [ ] Projection changes are announced through `aria-live="polite"`.
- [ ] The "does not place an order" note is programmatically associated with the dialog.

### S6 Brief

- [ ] The brief card has a heading and a readable reading order.
- [ ] Email delivery status is text, not an icon alone.
- [ ] The sample-data label is present when sample data is shown.

### S7 Settings and privacy

- [ ] Tabs are a real tablist with arrow-key navigation and `aria-selected`.
- [ ] Switches are native checkboxes with visible labels; mandatory ones announce why they are locked.
- [ ] Session revocation confirms and announces the result.
- [ ] Delete account confirmation names what will be deleted and what is retained.

### S8 Support and status

- [ ] Search has a label and announces the result count.
- [ ] The status card uses `status` and includes the last update time.
- [ ] A degraded state names the affected component in text.

## Testing approach

1. **Automated scan** — axe DevTools and Lighthouse on every core flow (catches ~30%).
2. **Keyboard-only** — complete onboarding, connect, drift, and export without a mouse.
3. **Screen reader** — VoiceOver on macOS and Safari; NVDA on Windows and Firefox.
4. **Zoom** — 200% text and 400% page zoom on every core flow.
5. **Colour** — simulate protanopia, deuteranopia, and tritanopia; confirm no meaning is lost.
6. **Reduced motion** — enable the preference and confirm transitions are removed.

## Launch gate

Zero blockers on the C13 EN 301 549 / WCAG 2.2 AA audit before launch. Any exception needs a
documented decision in `DECISIONS.md` and a remediation date.

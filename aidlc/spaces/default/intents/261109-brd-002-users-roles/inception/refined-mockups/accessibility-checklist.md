# Accessibility Checklist — Users, Roles & Permissions

Target: **WCAG 2.2 Level AA** on every BRD-002 surface. Each item names the surface it applies to and how the mockup meets it.

## Perceivable

| Requirement | Applies to | How the design meets it |
|---|---|---|
| 1.1.1 Non-text content has a text alternative | All screens | Icons carry `aria-label` or are decorative with `alt=""`/`aria-hidden`; the role and status dots are decorative, the labels carry meaning |
| 1.3.1 Info and relationships conveyed in structure | Settings, admin tables, metadata panel | Real headings in order; tables for tabular data; form labels associated with inputs; fieldsets for consent groups |
| 1.4.1 Use of colour is not the only signal | StatusPill, GrantBanner, errors | Every status colour is paired with a word and a dot; errors add written messages |
| 1.4.3 Contrast (minimum) | All screens | Parchment body pairs clear AA; meta text 4.64:1 on canvas; footnote-scale meta only on warm surfaces |
| 1.4.4 Resize text to 200% | All screens | Fluid layout, relative units; no fixed-height text containers |
| 1.4.10 Reflow at 320px | SPA settings and gates | Single column; tables become stacked cards |
| 1.4.11 Non-text contrast 3:1 | Inputs, buttons, focus ring | Component borders and focus rings meet 3:1 |
| 1.4.12 Text spacing | Settings, console | No clipping when spacing increases; line-height 1.6 |

## Operable

| Requirement | Applies to | How the design meets it |
|---|---|---|
| 2.1.1 Keyboard | All screens | Every control reachable and operable by keyboard; tables use native controls; drawer and modals trap focus and close on Escape |
| 2.1.2 No keyboard trap | Admin drawer, confirmation modals | Escape closes; focus returns to the trigger |
| 2.2.1 Timing adjustable | GrantBanner countdown | The countdown is informational; access expiry is a server rule, not a user task deadline; no form is time-limited |
| 2.4.1 Bypass blocks | App shell | Skip-to-content link first in tab order |
| 2.4.3 Focus order | All screens | Logical order; the grant banner is announced, not focused |
| 2.4.4 Link purpose | Settings, notices | Link text names its destination; no "click here" |
| 2.4.6 Headings and labels | All screens | Descriptive headings; labels above inputs |
| 2.4.7 Focus visible | All screens | Focus blue ring on every interactive element; never `outline: none` |
| 2.5.1 Pointer gestures | All screens | No path-based or multi-point gestures; drag alternatives not needed |
| 2.5.3 Label in name | Buttons | Visible label is part of the accessible name |
| 2.5.8 Target size (minimum) | Mobile screens | ≥24×24px minimum; ≥44×44px for primary actions and adjacent controls |

## Understandable

| Requirement | Applies to | How the design meets it |
|---|---|---|
| 3.1.1 Language of page | All screens | `lang` set per locale (en/vi) |
| 3.2.1 On focus / 3.2.2 On input | Settings, gates | No context change on focus or while typing; actions require explicit activation |
| 3.3.1 Error identification | Settings, grant reason, gates | Errors name the field and the fix in text |
| 3.3.2 Labels or instructions | All forms | Visible labels; reason length shown (1–500); country hint |
| 3.3.3 Error suggestion | Forms | Messages offer the correction ("Reason must be 1–500 characters") |
| 3.3.4 Error prevention (legal/financial/data) | Deletion, role change | Dedicated deletion page plus explicit confirm; role/status changes require a reason and are audited |
| 3.3.7 Redundant entry | Forms | Fields are not asked twice; provider-supplied values prefill |

## Robust

| Requirement | Applies to | How the design meets it |
|---|---|---|
| 4.1.2 Name, role, value | Custom components | Native elements first; RoleBadge/StatusPill are text; GrantBanner uses `role="status"` + `aria-live="polite"` |
| 4.1.3 Status messages | Save results, grant state | `aria-live="polite"` announcements without moving focus |

## Surface checklist

### Account settings & security
- [ ] Keyboard-only save and discard for every section
- [ ] Dirty-state message announced, not just shown
- [ ] Danger zone reachable and its consequence read in order
- [ ] 200% zoom keeps the sidebar usable (tabs on mobile)

### Delete account
- [ ] The what-happens and what-is-kept lists are real lists, read before the buttons
- [ ] The destructive confirm is not the first focusable control
- [ ] Recovery banner date is text, not only a countdown

### Sign-up gates
- [ ] Country select and checkboxes are labelled and keyboard-operable
- [ ] A blocked jurisdiction is announced as a status, not only styled
- [ ] No personal data is requested before the gates pass

### Support console
- [ ] GrantBanner announces grant and expiry once, not every tick
- [ ] The metadata panel's exclusion reminder is read with the panel heading
- [ ] Approval controls are disabled with an accessible explanation for self-approval

### Admin users
- [ ] Table headers are real `<th>` with scope; numeric columns right-aligned
- [ ] Drawer traps focus, Escape closes, focus returns to the row
- [ ] Reason-required save is explained before submission

## Testing

1. Automated: axe/Lighthouse on each screen (catches ~30%).
2. Keyboard-only pass through every flow in `interaction-spec.md`.
3. Screen reader pass: VoiceOver (macOS) and NVDA (Windows).
4. Zoom 200% and 400%; reflow at 320px.
5. Colour-blind simulation on status pills and the grant banner.

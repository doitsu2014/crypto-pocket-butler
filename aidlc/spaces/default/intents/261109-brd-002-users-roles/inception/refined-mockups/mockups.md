# Refined Mockups — Users, Roles & Permissions

Mid-fidelity structural mockups for the four BRD-002 surfaces. Each screen shows layout and content hierarchy, the five screen states, responsive notes, and the story it serves. Visual styling is intentionally absent: the Parchment Ledger tokens that fill it are mapped in `design-system-mapping.md`.

Classic scope skips the rough-mockups stage, so these are designed directly from `stories.md` and `requirements.md`.

## Conventions

- `[ ]` = interactive control; `( )` = radio; `[x]` = checkbox; `|` = table column edge.
- Annotations in `‹ ›` describe behaviour, not copy.
- Every screen lists its **empty**, **loading**, **error**, and **partial** states; the layout shown is the populated state.

## 1. Account settings & security (SPA)

**Persona:** Portfolio Owner · **Stories:** US1.4, US4.2, US4.3, US6.1

```
+--------------------------------------------------------------+
| Header: product mark            [Search]   [Avatar menu]     |
+------------------+-------------------------------------------+
| Sidebar (240px)  | Profile                                   |
| • Profile        | Display name  [____________________]      |
| • Preferences    | Email         name@example.com  [Verified]|
| • Consent        | Locale        [English (EN)      v]       |
| • Sessions       | Base currency [USD              v]        |
| • Privacy        | Timezone      [Asia/Ho_Chi_Minh  v]       |
|                  |                            [Save changes] |
|                  |                                           |
|                  | Notification preferences                  |
|                  | [ ] Daily brief   [ ] Alerts   [ ] Digest |
|                  |                                           |
|                  | Consent                                   |
|                  | Analytics  [On  ]  Withdrawn 2026-11-09    |
|                  |                                           |
|                  | Sessions                                  |
|                  | | Device        | Last seen | Action     |  |
|                  | | MacBook · SPA | today     | [Revoke]   |  |
|                  |                                           |
|                  | Danger zone                               |
|                  | Delete my account                        |
|                  | Deleting removes your access and starts  |
|                  | a 30-day recovery window.                |
|                  | [Delete account]  ‹goes to §1a›           |
+------------------+-------------------------------------------+
```

- **Roles and status** shown as read-only metadata beside the avatar: a **role badge** (Owner) and a **status pill** (Active). An admin also sees an `Admin` role badge; disabled/pending-deletion status replaces the pill with its label and a one-line explanation.
- **Save behaviour:** each section saves independently; a dirty section shows `Unsaved changes` and offers Discard.
- **States:** empty (profile fields blank only where the provider supplied none), loading (skeleton rows in each panel), error (inline under the field, with the value kept), partial (one section failed to load — that panel shows its own error and Retry while the rest stays usable).

### 1a. Delete account (dedicated page)

**Stories:** US4.3, US4.4

```
+--------------------------------------------------------------+
| ‹ Back to settings                                           |
|                                                              |
| Delete your account                                          |
|                                                              |
| What happens                                                 |
| • Your access ends immediately when you confirm.             |
| • You have 30 days to restore the account by signing in.     |
| • After 30 days your personal data is erased.                |
| • Records under a legal or statutory hold are kept.          |
|                                                              |
| What is kept                                                 |
| • Financial records required by retention policy.            |
|                                                              |
| Reason (optional)  [___________________________________]      |
|                                                              |
| [Cancel]                            [Delete my account]      |
|                                     ‹destructive, asks for   |
|                                       explicit confirmation› |
+--------------------------------------------------------------+
```

- **Confirmation:** the destructive button opens a confirmation step that repeats "this starts a 30-day recovery window" and names what is kept; the final action is a second explicit confirm, never a single click.
- **States:** a `pending_deletion` account sees this page replaced by a **recovery banner** on every authenticated screen: "Your account is scheduled for deletion. Restore it before <date>." with a `Restore account` primary action.
- **Restore:** signing in during the window shows the recovery banner; `Restore account` returns the account to active and confirms inline.

## 2. Sign-up gates (Visitor)

**Persona:** Anonymous visitor · **Stories:** US6.2, US6.3, US6.4, US6.5

```
+--------------------------------------------------------------+
|                       Product mark                            |
|                                                              |
|  Before you connect your first source                        |
|                                                              |
|  Country        [Vietnam                       v]            |
|  ‹jurisdiction check runs here, before any data is stored›   |
|                                                              |
|  [x] I am 18 or older                                        |
|  [x] I accept the Terms of Service                           |
|  [x] I have read the Risk Disclosure                         |
|                                                              |
|  [Continue]                                                  |
|                                                              |
|  We do not store your details until these steps pass.        |
+--------------------------------------------------------------+
```

- **Blocked jurisdiction:** the country select and Continue are replaced by a written notice naming that the service is unavailable in the selected country and that no data was stored. No personal data is collected on this screen.
- **Already signed in, newly restricted:** the owner sees a blocking **gate screen** with the same notice and a sign-out action; the block is audited server-side.
- **States:** loading (country list), error (check could not complete — retry, nothing stored), partial (terms version changed — re-acceptance required), success (continue to the first-source step).

## 3. Support console — grant and metadata panel

**Personas:** Support Agent, Admin · **Stories:** US3.1–US3.6, US2.4, US2.9

### 3a. Grant request (support agent)

```
+--------------------------------------------------------------+
| Support console                    Role: Support  [Sign out] |
+--------------------------------------------------------------+
| Find a user   [email or user id________________] [Search]     |
|                                                              |
| Grant request                                                |
| User          Mai Nguyen · user_01H...  ‹one user record›    |
| Reason        [____________________________________] (1–500) |
| Requested for ‹60 minutes maximum, set by the approver›      |
|                                                              |
| [Request access]                                             |
+--------------------------------------------------------------+
```

### 3b. Active grant banner (support agent, during access)

```
+--------------------------------------------------------------+
| ⏱ Access granted to Mai Nguyen — expires in 42 min            |
|   Reason: "Customer reports a missing connected account"      |
|   Metadata only. No balances, holdings, or credentials.       |
|                                                    [End now]  |
+--------------------------------------------------------------+
```

- The banner is the **grant banner** component: time remaining, the reason, and the metadata-only reminder, with an explicit end action. It persists across support screens while a grant is active.

### 3c. Metadata panel (support agent, under grant)

```
+--------------------------------------------------------------+
| Mai Nguyen                          Status: Active           |
| Role: Owner · Email: verified                                 |
|                                                              |
| Accounts and portfolios (metadata only)                      |
| | Name            | Type      | Sync status | Last synced   | |
| | Binance main    | Exchange  | Synced      | 12 min ago    | |
| | Main portfolio  | Portfolio | —           | —             | |
|                                                              |
| ‹No balances, holdings, values, transactions, orders,         |
|   credentials, or reset actions appear on this panel.›        |
+--------------------------------------------------------------+
```

- **States:** no grant (panel locked behind a "Request access" call to action), expired (banner closes, panel returns to locked with a one-line explanation), revoked (same, with "access ended by an admin"), loading/error per panel.
- **Admin view:** the same panel plus grant approval controls; an admin approving a grant sees the requester and reason and cannot approve their own request.

## 4. Admin user management

**Persona:** Admin · **Stories:** US2.4, US7.1, US7.2, US3.2, US3.5

```
+--------------------------------------------------------------+
| Admin · Users                    Role: Admin  [Sign out]     |
+--------------------------------------------------------------+
| [Search users by email or id_____________]  [Role v] [Status v]|
+--------------------------------------------------------------+
| | Email              | Role    | Status  | Last seen | • |    |
| | mai@example.com    | Owner   | Active  | today     | … |    |
| | an@example.com     | Admin   | Active  | today     | … |    |
| | support@example..  | Support | Disabled| 3 days    | … |    |
+--------------------------------------------------------------+
| Showing 1–20 of 214                          ‹pagination›     |
```

User record drawer (opens from a row):

```
+--------------------------------------------------------------+
| an@example.com                              [Close]          |
| Role      [Owner  v]   ‹change asks for a reason›             |
| Status    [Active v]   ‹Disable asks for a reason›            |
|           [Save]  ‹change is audited with actor and reason›   |
|                                                              |
| Support grants                                               |
| | Requester | Reason        | Expires | Action             |  |
| | Sofia Q.  | Ticket #4821  | 12 min  | [Approve] [Deny]   |  |
|                                                              |
| ‹No entitlement controls exist in this release.›              |
+--------------------------------------------------------------+
```

- **Self-approval rule:** the Approve control is disabled with an explanation when the requester is the signed-in admin.
- **States:** empty search result ("No users match — try an email or user id"), loading skeleton rows, error banner with Retry, partial (grants failed to load — the record stays usable, the grants panel offers Retry), disabled account (status pill shows Disabled with the reason and date).

## Responsive behaviour

| Surface | Mobile (<768px) | Tablet (768–1024px) | Desktop (>1024px) |
|---------|-----------------|---------------------|-------------------|
| Account settings | Single column, sections stacked, sidebar becomes a top tab bar | Two columns: tabs + content | Sidebar + content as drawn |
| Delete account | Full-width page, sticky action bar above the fold | Full-width card | Centred 640px card |
| Sign-up gates | Full-width form, country select full width | Centred 480px card | Centred 480px card |
| Support console | Read-only summary; grant request allowed; panel shows identifiers only, tables become stacked cards | Two columns | Table + panel as drawn |
| Admin users | Cards instead of a table; drawer becomes a full-screen sheet | Condensed table | Table + drawer as drawn |

## Screen-state matrix

| Screen | Empty | Loading | Success | Error | Partial |
|--------|-------|---------|---------|-------|---------|
| Account settings | Fields blank where the provider gave nothing; placeholders allowed | Skeleton rows | Saved confirmation inline | Field-level message, value kept | Per-panel error + Retry |
| Delete account | — | Submit pending | Recovery banner with date | Refused with the reason | — |
| Sign-up gates | Nothing stored until pass | Country list loading | Continue to first source | Check failed — retry, nothing stored | Terms re-acceptance |
| Support grant | No grant — locked panel | Request/approve pending | Active banner with countdown | Reason rejected (length), approval refused | Grant expired mid-session |
| Admin users | No users match | Skeleton rows | Saved change + audit note | Save refused, reason required | Grants panel failed to load |

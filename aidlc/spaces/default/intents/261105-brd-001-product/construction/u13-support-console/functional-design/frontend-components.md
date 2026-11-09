# Frontend Components — u13-support-console

React 18 + TypeScript 5 on Vite, Tailwind CSS v4 against Parchment Ledger tokens, TanStack Query
for server state, `@supabase/supabase-js` for auth. A separate static app under `support-console/`.
Session tokens in memory only.

## Hierarchy

```
SupportApp
├── SupportSignInGate          role check (support | admin)
├── SupportShell
│   ├── GrantBanner            reason, expiresAt, approver, "Request new grant"
│   ├── GrantRequestForm       target user, reason, duration, approver
│   ├── GrantApprovalQueue     pending grants; approve (second approver) or revoke (admin)
│   ├── AccountSearch          metadata search by account id or label
│   ├── AccountMetadataPanel   kind, venue, label, status, lastSyncedAt — no credentials
│   ├── TicketQueue            ticket metadata list
│   └── IncidentConsole
│       ├── IncidentForm       severity, summary, components
│       └── IncidentList       declared/investigating/resolved
└── StatusFooter               link to the public status page
```

## Props / state

| Component | Props | State |
|---|---|---|
| `SupportSignInGate` | `onReady(role)` | session, role, error |
| `GrantBanner` | `grant: {expiresAt, reason, approver, status}` | time-to-expiry |
| `AccountSearch` | `grantId`, `onSelect(accountId)` | query, results |
| `AccountMetadataPanel` | `accountId`, `grantId`, `reason` | query result, 403 reason |
| `TicketQueue` | `grantId` | status filter, ticket metadata |
| `IncidentForm` | `onDeclared(id)` | severity, summary, components, validation |

## Interaction flows

1. Sign in → role checked → shell; a missing role shows "not authorised" and no data.
2. Search → select an account → reason prompt (defaults to the grant reason) → metadata read.
3. Metadata read: skeleton → fields; a 403 shows "access ended" with the expiry time.
4. Incident: submit → confirm → C9 notification status shown ("sent", "pending", or an error with
   the reason).

## Form validation

- Reason: ≥ 20 characters, trimmed; the same validation as the server.
- Grant duration: 1–480 minutes.
- Second approver: required, must differ from the requester.
- Incident summary: ≤ 200 characters; severity is `sev1|sev2|sev3`.

## API integration

- One module owns all calls: `src/api/client.ts` (C2 base path, bearer from the in-memory session,
  RFC 7807 error mapping).
- The metadata read sends the free-text reason in `X-Support-Reason`, never in the URL. Search,
  metadata, and the ticket list require `grantId`; the ticket list exposes a pseudonymous
  `requesterRef`, never a requester label or conversation.
- No business math; no credential field exists anywhere in the DTOs or UI.

## Accessibility

- WCAG 2.2 AA, keyboard-first, visible focus, status colours paired with label and shape, financial
  values use tabular mono and never render a missing value as zero.
- A root-scoped customer service worker must not control `/support` or `/v1`; the console
  unregisters any registration whose scope covers `/support` at startup.

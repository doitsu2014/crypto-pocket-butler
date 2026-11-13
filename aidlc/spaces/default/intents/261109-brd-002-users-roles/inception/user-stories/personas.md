# Personas — Users, Roles & Permissions

Four personas carry the BRD-002 journeys. System is a non-user actor: scheduled work acts for no person and never appears as a persona in a story. Analyst/Viewer is deliberately excluded (post-MVP, FR-USR-10 marked Won't).

## P1 — Mai, the Portfolio Owner (primary)

**Role:** Retail crypto investor who uses the product for their own holdings.

**Goals:**
- Keep one trustworthy view of holdings spread across exchanges and wallets.
- Control her own profile, preferences, and privacy choices without contacting support.
- Be able to leave — export her data and delete her account — without losing the record she expects to keep under legal retention.

**Pain points:**
- Not knowing who at the provider or the platform can see her data.
- Deletion flows that either keep everything forever or lose records she is legally required to keep.
- Security settings hidden behind support tickets.

**Context:** Individual retail account; signs in with email and may use a social provider; may hold exchange credentials, which makes MFA mandatory.

**Tech comfort:** Medium — comfortable with a web app and mobile PWA, not with API keys or security terminology.

**Frequency:** Daily to weekly.

**Priority:** Primary persona; nearly every story is written from her point of view first.

## P2 — Arun, the Admin (secondary)

**Role:** Platform operator responsible for users, roles, registries, and support oversight.

**Goals:**
- Resolve access problems correctly: look up a user, change a role, disable an account.
- Approve or refuse support-access requests quickly, with a clear record of why.
- Keep every administrative action auditable and reversible in the record.

**Pain points:**
- Silent role changes that nobody can explain later.
- Tools that let one person request and approve the same access.
- Broad screens that expose portfolio contents when only metadata is needed.

**Context:** Staff account with `admin` in `app_metadata.role`; also a Portfolio Owner for their own holdings, so admin is additive.

**Tech comfort:** High.

**Frequency:** Daily.

**Priority:** Secondary; drives the administration and support-approval stories.

## P3 — Sofia, the Support Agent (secondary)

**Role:** Customer support agent who helps users with account questions.

**Goals:**
- See just enough account context to answer a question.
- Get temporary access quickly, with a reason, on the record.
- Never see or touch credentials, balances, or another person's portfolio.

**Pain points:**
- Being asked for information she is not permitted to see.
- Access that lingers after the ticket is closed.
- Screens that mix metadata and financial content.

**Context:** Staff account with `support` in `app_metadata.role`; access is only ever through an approved, time-bound grant for one user record.

**Tech comfort:** Medium.

**Frequency:** Daily during a shift.

**Priority:** Secondary; drives the support-grant journey and the metadata boundary.

## P4 — Visitor, the Anonymous Visitor (entry)

**Role:** Prospective user who has not signed in.

**Goals:**
- Understand what the product does and whether it is available in their country.
- Sign in or create an account without exposing personal data before the gates pass.

**Pain points:**
- Signing up and only then learning the service is unavailable where they live.
- Being asked for personal data before knowing the terms.

**Context:** No token; sees public pages, the sign-in flow, and the pre-sign-up gates only.

**Tech comfort:** Low to medium.

**Frequency:** Once, at entry.

**Priority:** Entry persona; drives the terms, age, and jurisdiction gates.

## Persona Relationships

- **P4 Visitor → P1 Owner:** the visitor becomes an owner after sign-in and the gates pass. No personal data is stored before both.
- **P1 Owner ← P3 Support:** Support sees a user's *metadata* only, and only under an approved grant. Never the owner's portfolio contents or credentials.
- **P2 Admin → P1 Owner:** an admin may manage roles, statuses, and support grants; an admin may not read another user's portfolio data. Admin is additive only on their own resources.
- **System:** runs scheduled jobs over users' data; it is an actor for authorisation rules but never a persona, and its stories are expressed as owner-side guarantees (no data appears under the wrong owner).

## Priority Ranking

1. **P1 Portfolio Owner** — the product's reason to exist; most Must stories.
2. **P3 Support Agent** — highest security sensitivity after the owner, and the most elaborate permission model.
3. **P2 Admin** — administrative control and audit; fewer stories, high consequence.
4. **P4 Visitor** — a small set of gate stories with high legal weight.

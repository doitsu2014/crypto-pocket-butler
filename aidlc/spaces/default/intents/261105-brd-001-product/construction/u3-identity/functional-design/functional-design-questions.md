# Functional Design Questions — u3-identity

These questions settle the choices the requirements, stories, and contracts leave open for the
Identity service. Answer each with a letter (A–D or X).

## Q1. [Architect] How should the internal Identity user be created?

Supabase Auth owns sign-up and sign-in, but the internal `User` row (role, status, locale, base
currency, timezone) must exist before the gateway can resolve a principal through Identity.

- A. Just-in-time: Identity upserts the internal user from the validated JWT on the first authenticated request.
- B. Signed Supabase Auth webhook: Identity creates or updates the user before first use.
- C. Both — the webhook is the primary path, with a JIT upsert as the fallback when the webhook has not arrived.
- D. An admin provisioning step creates the internal user before anyone can sign in.
- X. Other (please specify)

[Answer]: A — Just-in-time: Identity upserts the internal user from the validated JWT on the first authenticated request.

## Q2. [Architect] Who owns session listing and revocation?

FR1.4 requires listing a user's devices and sessions, showing device, location, and last
activity, and revoking any of them. Supabase Auth already holds the sessions.

- A. Supabase Auth owns sessions; Identity proxies its admin API for list and revoke.
- B. Identity records its own device/session rows per sign-in; revocation blocks at the gateway.
- C. Both — Supabase Auth is the source of truth and Identity caches a device view.
- D. Identity does not own this; the SPA calls Supabase Auth directly.
- X. Other (please specify)

[Answer]: A, revised after the design review — revocation calls Supabase Auth's admin API; listing uses an Identity-owned DeviceSession view, because the Auth admin API exposes no session list.

## Q3. [Security engineer] Where is MFA enforced?

FR1.2 and NFR10 require MFA for any user with stored exchange credentials, and story US7.1
requires enrolment before portfolio data is shown.

- A. Supabase Auth AAL2; Identity refuses the principal and the gateway refuses portfolio access while an exchange-credential user's session is AAL1.
- B. Identity implements its own TOTP and recovery codes.
- C. The SPA enforces it; services trust the session.
- D. Not enforced in this release.
- X. Other (please specify)

[Answer]: A — Supabase Auth AAL2; Identity refuses the principal and the gateway refuses portfolio access while an exchange-credential user's session is AAL1.

## Q4. [Architect] How does a data export run and deliver?

FR8.1 and US7.3 require a machine-readable export of accounts, portfolios, transactions, and
snapshots, with progress shown for a large export (AC7.3.2).

- A. Asynchronous job: Identity records the request, calls each service, assembles one archive, and offers a signed download when ready with progress.
- B. Synchronous stream assembled during the request (may time out for large data).
- C. Per-service downloads handed back as a list.
- D. Assemble, then email a download link.
- X. Other (please specify)

[Answer]: A — Asynchronous job: Identity records the request, calls each service, assembles one archive, and offers a signed download when ready with progress.

## Q5. [Compliance specialist] What does account deletion do to personal data?

FR8.2, NFR13, and US7.4 require deletion or anonymisation while legally retained financial
records stay labelled under their legal basis.

- A. Anonymise personal fields in every service, keep labelled legal-retention records, and destroy the per-subject crypto-shredding key in Vault.
- B. Hard-delete personal rows everywhere and keep only the legal minimum.
- C. Soft-delete the account only; data remains until a manual purge.
- D. Delete everything, including financial records.
- X. Other (please specify)

[Answer]: A — Anonymise personal fields in every service, keep labelled legal-retention records, and destroy the per-subject crypto-shredding key in Vault. (Revised from a soft-delete answer after the FR8.2 contradiction.)

## Q6. [Security engineer] Who is the second approver for a support-access grant, and what is the default time bound?

FR11.2 and US9.3 require a support session that is read-only, time-bound, reason-recorded,
audited, and approved by a second person.

- A. A user with the admin role, distinct from the requester; default 60 minutes.
- B. Any user with the support role, distinct from the requester; default 60 minutes.
- C. The account owner (the end user); default 24 hours.
- D. A support user of the same role, self-approved with a recorded reason; default 60 minutes.
- X. Other (please specify)

[Answer]: A — A user with the admin role, distinct from the requester; default 60 minutes.

## Q7. [Compliance specialist] Where is the jurisdiction and age gate enforced?

FR8.4 and US7.6 require the gate before any personal data is collected.

- A. Before sign-up: the SPA asks jurisdiction and age and the gateway/Identity refuses account creation before data reaches Supabase Auth.
- B. At first sign-in, after Supabase Auth has created the account; Identity blocks access.
- C. In Supabase Auth with a pre-sign-up hook.
- D. Informational only; no hard gate.
- X. Other (please specify)

[Answer]: A, implemented with the option-C enforcement point after the design review — the SPA asks jurisdiction and age before sign-up, Identity returns a signed gate token, and Supabase Auth's pre-sign-up hook verifies it so a sign-up cannot bypass the gate.

## Q8. [Architect] How is consent recorded?

FR8.3 requires consent for marketing and non-essential analytics with no pre-ticked boxes, and
C3 exposes current consent by purpose.

- A. Current state per purpose plus an append-only history of every change.
- B. Only the current state per purpose.
- C. Append-only events with the current state derived.
- D. Marketing only; analytics is treated as essential.
- X. Other (please specify)

[Answer]: A — Current state per purpose plus an append-only history of every change.

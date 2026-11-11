# Requirements Analysis — BRD-002 Questions

BRD-002 and the existing code answer most of what this stage needs. These questions settle the points they leave open or contradict, so the requirements can be written without guessing. Answer each with a letter after `[Answer]:` (for example `[Answer]: A`), or pick `X` and say what you want instead.

## Q1. [product manager] User statuses: implement all four the BRD declares, or keep the three in the code?

BRD-002 FR-USR-20 declares four statuses — `active`, `disabled`, `pending_deletion`, `deleted` — but the Identity schema stores only `pending`, `active`, `deleted`, so `disabled` and `pending_deletion` do not exist today.

- A. Implement all four statuses as BRD-002 declares
- B. Keep the three implemented statuses and record the narrower set as a deliberate decision for this release
- C. Implement `disabled` now and defer `pending_deletion` until the BRD-017 deletion work
- X. Other (please specify)

[Answer]: A — Implement all four statuses as BRD-002 declares.

## Q2. [product manager] Soft-delete recovery window: how long can a deleted account be restored?

BRD-002 leaves this open (OQ-002). The answer sets how long the data stays recoverable before permanent erasure.

- A. 30 days
- B. 7 days
- C. 90 days
- D. No recovery window; deletion is immediate after confirmation (legal holds aside)
- X. Other (please specify)

[Answer]: A — 30 days.

## Q3. [product manager] Deletion cascade: does BRD-017 own the policy, or do we pin it here?

BRD-002 FR-USR-18 requires a per-entity deletion cascade and BR-USR-13 preserves records under legal hold or statutory retention, but the concrete cascade lives in BRD-017 (privacy and data rights), which has not been worked yet.

- A. BRD-017 owns the cascade and retention policy; BRD-002 requirements reference it and pin only that legal holds override
- B. Pin the full per-entity cascade in this requirements set now
- C. Cover account-level deletion in this release and add the per-entity cascade when BRD-017 is worked
- X. Other (please specify)

[Answer]: A — BRD-017 owns the cascade and retention policy; BRD-002 references it and pins only that legal holds override.

## Q4. [security engineer] Admin role claim: exact name and scope?

BRD-002 leaves the exact role claim open (OQ-001). The code reads `app_metadata.role` and treats `admin` and `support` specially, with any other value as a plain user.

- A. `app_metadata.role`, realm-wide, with exact values `admin` and `support`; any other value is a plain user
- B. The same values, but client-scoped rather than realm-wide
- C. A different claim name or path (specify it)
- X. Other (please specify)

[Answer]: A — `app_metadata.role`, realm-wide, with exact values `admin` and `support`; any other value is a plain user.

## Q5. [security engineer] Support access: what is the maximum window, and who approves it?

BRD-002 requires time-bound, reason-recorded support access but gives no duration. The code caps a grant at 60 minutes and requires a different admin to approve it.

- A. At most 60 minutes per grant, approved by a different admin, reason of 1–500 characters, every access audited
- B. At most 24 hours per grant, with the same approval and audit rules
- C. At most 60 minutes, but an admin may extend a grant once
- D. No fixed maximum; a grant expires when its ticket closes
- X. Other (please specify)

[Answer]: A — At most 60 minutes per grant, approved by a different admin, reason of 1–500 characters, every access audited.

## Q6. [compliance specialist] What exactly may Support see under a grant?

BRD-002 says Support may read "account metadata" and nothing else, but the field list is not defined, so the boundary is not testable as written.

- A. Account and portfolio identifiers, labels, source type, sync status and timestamps, plus the user's display name and email-verification status — never holdings, balances, values, transactions, orders or credentials
- B. Account and portfolio identifiers and sync status only; no user profile fields
- C. The user's profile plus account labels and sync state; still never credentials or portfolio contents
- X. Other (please specify)

[Answer]: A — Account and portfolio identifiers, labels, source type, sync status and timestamps, plus display name and email-verification status — never holdings, balances, values, transactions, orders or credentials.

## Q7. [compliance specialist] Jurisdiction restrictions: which jurisdictions, and how is location detected?

BRD-002 FR-USR-16 requires jurisdiction restrictions at sign-up and on continued use, but does not name the restricted jurisdictions or the detection method. The launch markets are Vietnam plus the EU/UK.

- A. Block only the jurisdictions on a maintained sanctions/regulatory list; detect by self-declared country at sign-up plus IP geolocation
- B. Self-declared country only
- C. IP geolocation only
- D. No jurisdiction gating at launch beyond the sign-up confirmation
- X. Other (please specify)

[Answer]: A — Block only the jurisdictions on a maintained sanctions/regulatory list; detect by self-declared country at sign-up plus IP geolocation.

## Q8. [compliance specialist] Age assurance: self-declaration, or verification in some markets?

BRD-002 FR-USR-15 requires age confirmation before the first source, and OQ-004 asks which launch markets need verification beyond self-declaration.

- A. Self-declaration only at launch, in every market
- B. Self-declaration, plus age verification only in the markets that legally require it (name them)
- C. Verification in all launch markets
- X. Other (please specify)

[Answer]: A — Self-declaration only at launch, in every market.

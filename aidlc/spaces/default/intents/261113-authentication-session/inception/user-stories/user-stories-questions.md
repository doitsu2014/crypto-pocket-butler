# User Stories — Plan and Questions

## Story plan

**Format:** every story follows `As a [persona], I want [action], so that [benefit]` with INVEST
notes, a MoSCoW priority, and 3–6 Given/When/Then acceptance criteria (`AC{group}.{seq}.{n}`)
including at least one error or edge path.

**Breakdown approach:** by capability area, so each story cuts through SPA, gateway and Identity
together rather than by technical layer:

1. Sign-in and the sign-in page
2. Session lifecycle (refresh, expiry, sign-out, in-memory tokens)
3. MFA and the limited session
4. Sign-up gates and email verification
5. Device and session management
6. Login alerts
7. Account recovery
8. Edge reachability (the gateway route table)
9. Token trust and outage behaviour

**Personas:** the draft proposes four — the account holder, the same holder in trouble, the
support agent, and the administrator. The questions below settle whether that set is right and how
granular the stories should be.

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [product manager] Is the proposed persona set right?

The draft proposes four personas: **the account holder** (day-to-day sign-in and settings), **the
account holder in trouble** (locked out, or suspicious of a device), **the support agent** (runs an
audited recovery), and **the administrator** (acts on another user's sessions).

- A. Yes — keep all four; the "in trouble" persona deserves its own because the needs differ sharply
- B. Three only — fold "in trouble" back into the account holder
- C. Five — also split out the compliance/ops engineer who owns the jurisdiction list
- D. Two only — the account holder and the internal operator
- X. Other (please specify)

[Answer]: A — keep all four personas. **Mode:** guided

## Q2. [product manager] How granular should the stories be?

Standard depth suggests stories completable in 1–3 days.

- A. One story per user-visible outcome, roughly 22–28 stories across the nine areas
- B. Finer — split each area into per-rule stories, roughly 40+ stories
- C. Coarser — one story per area, about 9 stories with many criteria each
- D. Let the story count fall where the requirements do, with no target
- X. Other (please specify)

[Answer]: A — one story per user-visible outcome, roughly 22–28 stories across the nine areas. **Mode:** guided

## Q3. [quality engineer] How should the deferred sign-in methods appear?

Google/Apple OAuth and passkeys are deferred to a follow-up intent (requirements Q1).

- A. List them as Won't Have stories so the backlog records the decision and the reason
- B. Omit them entirely from `stories.md` and keep the exclusion only in the requirements
- C. Include them as Could Have with no acceptance criteria, as placeholders
- D. Record them once as a single "deferred sign-in methods" note rather than as stories
- X. Other (please specify)

[Answer]: A — list them as Won't Have stories so the backlog records the decision and the reason. **Mode:** guided

## Q4. [developer] How should the route-table work be expressed?

The practice says a slice is complete when it is reachable through the gateway's declared routes.
Four auth paths are undeclared today: the signup gate, device-session list, device-session revoke,
and the MFA enrol/recovery paths the SPA calls.

- A. Fold reachability into each feature story as an acceptance criterion, plus one story for the
  SPA calls that have no owning handler at all
- B. One dedicated "declare the auth routes at the gateway" story, and let feature stories assume it
- C. Both — the dedicated story for the declaration work, and a reachability criterion in every
  story it unblocks
- D. Handle it entirely in Contract Design, not in stories
- X. Other (please specify)

[Answer]: A — fold reachability into each feature story as a criterion, plus one story for the SPA calls that have no owning handler at all. **Mode:** guided

## Q5. [compliance specialist] How should the accepted revocation window be recorded in the stories?

The BRD says a revocation takes effect on the next request; this intent accepts a residual window
of up to one hour (requirements Q4).

- A. State it in the revoke story as a documented limit with its own acceptance criterion, so the
  behaviour is tested rather than assumed
- B. Keep it out of stories; it is a requirements-level note
- C. Make closing the window its own Could Have story
- D. State it in the revoke story and add a Could Have story to close the window later
- X. Other (please specify)

[Answer]: A — state it in the revoke story as a documented limit with its own acceptance criterion, so the behaviour is tested rather than assumed. **Mode:** guided

## Q6. [product manager] What should the priority mix be?

- A. Almost everything Must Have; the BRD marks 18 of 22 functional requirements Must
- B. Must Have only for what the BRD marks Must **and** is reachable end to end; everything else
  Should Have
- C. Must Have for the sign-in, session and device core; Should Have for MFA, gates, alerts and
  recovery
- D. Let each story's priority follow its BRD row exactly
- X. Other (please specify)

[Answer]: D — each story's priority follows its BRD row exactly. **Mode:** guided

# User Stories — Assessment

## Decision

**Execute.**

## Rationale

BRD-003 is a user-facing security surface with several distinct actors whose needs differ enough
that a requirement list alone would hide them behind one voice:

- an ordinary person signing up, signing in, and managing their own devices;
- the same person at a moment of trouble — locked out, or suspicious of a device they do not
  recognise;
- a support agent performing a recovery under supervision;
- an administrator acting on another user's sessions.

The behaviour is also policy-heavy: MFA tiers, a jurisdiction gate, an age gate, a limited session,
and a revocation rule whose residual window was accepted deliberately. Those rules are easier to
hold together as stories with Given/When/Then criteria than as numbered requirements.

## Factors considered

| Factor | Observation | Weight |
|---|---|---|
| Project type | Brownfield; the sign-in path exists and is partly built | Execute — stories must describe finishing existing behaviour, not new invention |
| User-facing scope | High: sign-in, sign-up, MFA, sessions, recovery, alerts are all screens | Execute |
| Personas | Four distinct actors, two of them internal | Execute |
| Business logic | Policy rules (MFA tier, gates, revocation, limited session) | Execute |
| Cross-team work | Identity, gateway and SPA must change together | Execute — stories must cut across layers |
| Requirements quality | 57 functional sub-requirements and 10 NFRs, all traced | Sufficient source for stories without re-elicitation |

## Where stories add the most value

1. **The sign-up gate chain** — terms, risk, age, jurisdiction, then account creation. Four rules
   that must happen in one order, before provisioning, enforced on the trusted side.
2. **The limited session** — a single story that ties MFA policy to what the SPA may show and what
   the API may serve.
3. **Device management** — list, individual revoke with a confirmation when it is the current
   session, revoke-all, and the audit trail behind each.
4. **Recovery** — the support-assisted path, where the interesting requirement is what must *not*
   leak at each step.
5. **The route-table closure** — the practice-affirmed completion test: a story per auth path that
   is currently unreachable through the gateway.

## Approach

Stories are organised by capability area (sign-in, session lifecycle, MFA, gates and verification,
devices, alerts, recovery, edge reachability), not by screen, so each story can cut through SPA,
gateway and service together. Each story carries 3–6 Given/When/Then criteria including at least
one error or edge path, per the team's affirmed testing posture.

Requirements coverage is recorded element by element in `traceability.json`.

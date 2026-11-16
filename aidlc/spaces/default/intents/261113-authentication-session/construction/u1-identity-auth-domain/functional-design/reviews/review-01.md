## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-11-14T11:10:11Z
**Iteration:** 1
**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | aidlc/spaces/default/intents/261113-authentication-session/construction/u1-identity-auth-domain/functional-design/entities.md > entity InternalUser attribute first_signed_in_at; rules.md > BR1.3; functional-spec.md > Session refresh and absolute expiry | The 30-day absolute cap is anchored to InternalUser.first_signed_in_at, which entities.md declares immutable after initial provisioning. For any account older than 30 days the cap is permanently exceeded, and a fresh sign-in cannot reset it without violating immutability, so long-lived users would be locked out of refresh forever. The anchor must be per-session, not per-user-creation. | Move the absolute-cap anchor off InternalUser onto a per-session timestamp (e.g. DeviceSession first_seen_at or a new session-created anchor), define exactly what resets it on fresh sign-in versus refresh, and remove or re-scope the immutability constraint on InternalUser.first_signed_in_at. | New |
| R-02 | Major | aidlc/spaces/default/intents/261113-authentication-session/construction/u1-identity-auth-domain/functional-design/entities.md > entity MfaPolicyState; functional-spec.md > MfaPolicyState transitions | The state-transition list uses states not_enrolled and enrolment_pending, but the MfaPolicyState entity carries only booleans mfa_required/mfa_enrolled plus an aal enum of aal1/aal2, so a developer cannot map enrolment_pending to any persisted field and cannot distinguish not-started from in-progress enrolment. | Add the missing enrolment-pending representation to MfaPolicyState (an explicit status field or defined boolean combination) and align the transition list with the persisted attributes, including what a retry reads. | New |
| R-03 | Major | aidlc/spaces/default/intents/261113-authentication-session/construction/u1-identity-auth-domain/functional-design/entities.md > entity MfaPolicyState attribute exchange_credential_present; contract-summary.md > C1 get_principal | MfaPolicyState.exchange_credential_present drives access restriction for exchange-credential users, but no upstream contract gives Identity that fact: C1 defines no port or caller supplying it, and the one-schema-per-service rule forbids Identity reading another service's tables, leaving an undeclared cross-service dependency with no owner, freshness, or failure behavior. | Name the authoritative source of the exchange-credential signal, add the port or contract operation Identity uses to obtain it (or state it arrives via an existing C5 operation), and define staleness and outage behavior for that read. | New |
| R-04 | Major | aidlc/spaces/default/intents/261113-authentication-session/construction/u1-identity-auth-domain/functional-design/functional-spec.md > First-seen-device login alert; entities.md > DeviceSession and LoginEvent | Two stores both record per-device history (DeviceSession rows and LoginEvent rows) and the spec says Identity compares against known device history without naming which store is authoritative, so concurrent first-seen sign-ins could double-alert and later reads could disagree. | Declare one authoritative device-history store for the first-seen decision, state how the other is derived or reconciled, and define the dedupe key and race behavior for concurrent first-seen sign-ins. | New |
| R-05 | Minor | aidlc/spaces/default/intents/261113-authentication-session/construction/u1-identity-auth-domain/functional-design/entities.md > entity DeviceSession attribute is_current | is_current is a required stored boolean, yet the entity constraint says it is derived from the validated caller context rather than request input, so every stored value goes stale as soon as the caller changes device and every list response must recompute it anyway. | Make is_current a derived response-only marker rather than a persisted attribute, or define the write/refresh rule that keeps the stored value correct. | New |
| R-06 | Minor | aidlc/spaces/default/intents/261113-authentication-session/construction/u1-identity-auth-domain/functional-design/functional-spec.md > Signup gate and provisioning authorization | Jurisdiction comes from an unnamed trusted source or an undescribed trusted-side verification of the browser value, so a developer cannot implement the check: no source, no verification rule, and no behavior when the trusted source is unavailable. | Name the trusted jurisdiction source (or the verification rule applied to the submitted value) and the outage/undetermined behavior, or record it as an explicit U1 open item owned with a default-deny rule. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| none named in dispatch | No stage validation tool was run | No mechanical cross-reference or cycle check was available for this pass; findings rest on manual contract comparison |

### Summary

The domain policy is coherent and traceable, but the session-cap anchor on an immutable per-user timestamp would permanently expire every account older than 30 days, and three ownership and state-mapping gaps leave the developer guessing. Fix the anchor and the MFA, credential-signal, and device-history questions, and this becomes implementable.

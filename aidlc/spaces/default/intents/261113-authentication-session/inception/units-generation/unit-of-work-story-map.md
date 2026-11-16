# Unit of Work Story Map — Authentication & Session Management

Every upstream story is assigned to the Unit that implements its acceptance criteria. Additional Units
listed in the same row provide dependencies or integrated proof, but do not take primary ownership.
Deferred stories are explicitly marked as out of scope for this intent.

| Story ID | Unit ID | Unit directory | Supporting / integration Unit(s) | Coverage note |
|---|---|---|---|---|
| US8.1 | U2 | `u2-identity-gateway` | U4 | Declare auth routes and owning handlers; U4 checks set completeness |
| US1.1 | U3 | `u3-spa-auth-sessions` | U2, U4 | Password sign-in via Supabase Auth; gateway needed for protected follow-up |
| US1.2 | U3 | `u3-spa-auth-sessions` | U2, U4 | Magic-link callback and PKCE handling |
| US1.3 | U3 | `u3-spa-auth-sessions` | U2 | Protected-page return path |
| US2.1 | U1 | `u1-identity-auth-domain` | U2, U3 | Token policy and refresh lifecycle; 30-day anchor remains open in requirements |
| US2.2 | U3 | `u3-spa-auth-sessions` | U2 | Redirect after unusable refresh token |
| US2.3 | U3 | `u3-spa-auth-sessions` | U4 | Memory-only storage; U4 checks runtime/browser behaviour |
| US2.4 | U1 | `u1-identity-auth-domain` | U2, U3 | Provider and app-session termination; SPA clears local memory |
| US3.1 | U1 | `u1-identity-auth-domain` | U2, U3 | MFA policy and enrolment use case; provider adapter in U2 |
| US3.2 | U1 | `u1-identity-auth-domain` | U2, U3 | AAL2 policy and limited-session decision |
| US3.3 | U1 | `u1-identity-auth-domain` | U2, U3 | Step-up rule and revocation after sensitive change |
| US4.1 | U1 | `u1-identity-auth-domain` | U2, U3 | Trusted-side consent gate decision |
| US4.2 | U1 | `u1-identity-auth-domain` | U2, U3 | Trusted-side jurisdiction decision |
| US4.3 | U1 | `u1-identity-auth-domain` | U2, U3 | Email-verification ownership before exchange credential |
| US5.1 | U1 | `u1-identity-auth-domain` | U2, U3 | Identity-owned device/session view |
| US5.2 | U1 | `u1-identity-auth-domain` | U2, U3 | Provider-first revocation and residual-window behavior |
| US5.3 | U1 | `u1-identity-auth-domain` | U2, U3 | Bulk revoke-all application use case |
| US5.4 | U1 | `u1-identity-auth-domain` | U2, U3 | Revoke other sessions after credential change |
| US6.1 | U1 | `u1-identity-auth-domain` | U2, U4 | Identity decides alert eligibility; U4 verifies Notification delivery |
| US7.1 | U1 | `u1-identity-auth-domain` | U2, U3 | Recovery-case creation and support authorization |
| US7.2 | U1 | `u1-identity-auth-domain` | U2, U3 | Recovery completion, provider session changes and audit |
| US7.3 | U1 | `u1-identity-auth-domain` | U2, U3 | Non-revealing recovery behaviour |
| US9.1 | U2 | `u2-identity-gateway` | U4 | JWT validation and protected route policy |
| US9.2 | U1 | `u1-identity-auth-domain` | U2 | Disabled-user principal policy and lazy creation |
| US9.3 | U1 | `u1-identity-auth-domain` | U2, U3, U4 | Provider-outage policy across SPA, Identity and gateway |
| US10.1 | — | — | — | Deferred by requirements Q1; Won't Have this intent |
| US10.2 | — | — | — | Deferred by requirements Q1; Won't Have this intent |

## Unit coverage

- U1 implements Identity-owned policy and lifecycle stories: MFA, gates, sessions, recovery, alert
  eligibility, disabled-user policy and outage behaviour.
- U2 supplies the required Identity HTTP/infrastructure adapters and declared gateway route/JWT
  boundary.
- U3 implements the SPA surfaces, session presentation and callback behaviour.
- U4 proves the integrated route invariant and browser/API journeys.
- US10.1 and US10.2 remain explicitly deferred and are not implementation gaps for this intent.

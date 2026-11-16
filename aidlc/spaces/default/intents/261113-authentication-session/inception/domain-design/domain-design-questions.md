# Domain Design — Questions

The domain-design stage defines logical code components and entity ownership only. It does not choose
which service deploys them or specify schemas, field types, API methods, cache policy, or token
lifetimes; those details belong to later design stages.

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [architect] Where should the new auth flows live?

Today Supabase Auth owns credentials and provider sessions; Identity owns the internal user and device
session view; the SPA owns the sign-in experience; the gateway validates tokens and routes calls.
Should this intent extend those existing owners, or create another logical component?

- A. Extend the existing SPA Authentication, API Gateway, and Identity components; Supabase Auth
  stays the external identity provider (recommended)
- B. Create a separate Authentication component inside Identity, distinct from User Registry and
  Device Sessions
- C. Create a new Auth service boundary separate from Identity
- D. Split each capability into independent auth, MFA, recovery, and session components
- X. Other (please specify)

[Answer]:

## Q2. [architect] Who owns device-session state?

The code already has an Identity `SessionService`, device-session repository port, and Supabase Auth
admin adapter. The provider can revoke a session, but the Identity view supplies the session list.

- A. Identity Device Sessions owns the app's device/session view and revocation workflow; Supabase
  Auth remains authoritative for provider session validity (recommended)
- B. Supabase Auth owns all device state; Identity only proxies provider responses
- C. Split listing to Identity and revocation to the gateway
- D. Add a new Session Management component separate from Identity
- X. Other (please specify)

[Answer]:

## Q3. [architect] Where does sign-up gate policy belong?

Identity already serves the signup gate and owns jurisdiction decisions. The SPA gathers terms, risk,
and age choices, but the current page supplies fixed values.

- A. Identity Signup Gates owns trusted-side evaluation and the gate record; the SPA owns collecting
  user choices; the gateway routes to Identity (recommended)
- B. Put gate policy and record ownership in the SPA; Identity only provisions the user
- C. Make a new Compliance service own all consent and jurisdiction decisions
- D. Put the gate record in the API Gateway because it is the entry point
- X. Other (please specify)

[Answer]:

## Q4. [architect] What is the logical boundary for support-assisted recovery?

The accepted recovery path is support-assisted and audited. Identity already owns support grants,
authorisation and user lifecycle; the support console is a separate frontend.

- A. Identity Recovery owns the recovery case and coordinates existing Support Grants, Auth Admin,
  session revocation and audit; Support Console remains the human interface (recommended)
- B. Support Console owns the recovery state and calls Identity for each step
- C. Add a separate Recovery service and schema
- D. Treat recovery as a new operation inside Device Sessions with no distinct case lifecycle
- X. Other (please specify)

[Answer]:

## Q5. [architect] How should login alerts cross component boundaries?

The alert is an in-app notification from Identity's sign-in event to the existing Notification
service. The project forbids message queues and uses bounded synchronous internal HTTP.

- A. Identity owns the decision that a sign-in is new/unusual; it calls Notification through an
  inward port, with bounded retry and an idempotency key (recommended)
- B. The gateway detects new devices and calls Notification
- C. The SPA decides whether a device is new and creates the notification
- D. Add an event bus despite the current no-queue rule
- X. Other (please specify)

[Answer]:

## Q6. [architect] Should MFA policy be a separate domain component?

Identity already owns account status and authorisation; MFA provider factors are managed by Supabase
Auth, while the app decides when AAL2 is required and what a limited session may access.

- A. Identity MFA Policy owns the app's requirement and enrolment state; it delegates factor operations
  to Supabase Auth and informs the gateway/Identity access checks (recommended)
- B. The SPA owns the policy and services trust its limited-session flag
- C. The gateway owns all MFA policy, including factor enrolment
- D. A new MFA service owns factors and policy independently of Identity
- X. Other (please specify)

[Answer]:

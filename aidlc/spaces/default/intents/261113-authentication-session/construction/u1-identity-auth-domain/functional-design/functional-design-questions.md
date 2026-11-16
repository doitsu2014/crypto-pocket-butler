# Functional Design — U1 Identity Auth Domain Questions

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [security engineer] Which event starts the 30-day absolute session cap?

The requirements still list this as open, but story AC2.1.3 says the cap is measured from first sign-in. This determines whether refreshing can extend a session beyond 30 days.

- A. First sign-in; refresh never resets the 30-day cap
- B. Most recent refresh; each refresh starts a new 30-day cap
- C. Each provider session has its own 30-day cap, independent of account sign-in history
- D. Defer the anchor decision and do not finalize the session-expiry rule in U1
- X. Other (please specify)

[Answer]: A. First sign-in; refresh never resets the 30-day cap. **Mode:** guided

## Q2. [compliance specialist] Who may approve a support-assisted recovery?

The requirements select support-assisted recovery but leave its approver open. The contract proposes a distinct approver under the existing support-grant policy, but marks this unresolved.

- A. Reuse the existing support-grant rule: a different authorized administrator approves the recovery
- B. Require a dedicated recovery-approver role, separate from support-grant administration
- C. Require two distinct authorized administrators to approve every recovery
- D. Keep recovery approval unresolved; U1 must not encode an approver policy
- X. Other (please specify)

[Answer]: A. Reuse the existing support-grant rule: a different authorized administrator approves the recovery. **Mode:** guided

## Q3. [security engineer] What location signal should trigger a new-login alert in U1?

The requirements mention a new device or unusual location, while the contract says only first-seen devices are actionable until a location rule is specified. This choice sets the Identity alert-eligibility policy.

- A. First-seen device only; leave unusual-location detection out of U1
- B. First-seen country compared with the user's prior sign-ins
- C. First-seen city compared with the user's prior sign-ins
- D. Implausible travel speed between sign-ins, using a threshold to be specified
- X. Other (please specify)

[Answer]: A. First-seen device only; leave unusual-location detection out of U1. **Mode:** guided

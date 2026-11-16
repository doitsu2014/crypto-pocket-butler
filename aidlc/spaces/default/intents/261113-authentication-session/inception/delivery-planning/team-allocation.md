# Team Allocation — Authentication & Session Management

A **Bolt** is one build pass over a Unit that ends in a running result. A **mob** is the AI lead and
specialist support that work on a Bolt. Team Formation was skipped by the `classic` scope, so no
human team roster exists; every Unit is AI-executed with a human checkpoint approval. Unit responsibilities are defined in `unit-of-work.md`; direct dependencies and the strict chain are defined in `unit-of-work-dependency.md`.

| Bolt | Unit | Lead | Supporting expertise | Human checkpoint |
|---|---|---|---|---|
| Bolt 1 | U1 `u1-identity-auth-domain` | `aidlc-developer-agent` | Architect for ports/boundaries; Quality for use-case and failure-path tests; DevSecOps for auth policy and data handling | Approve U1 after its applicable design, code and verification are complete |
| Bolt 2 | U2 `u2-identity-gateway` | `aidlc-developer-agent` | Architect for route/contract alignment; DevSecOps for JWT, rate limits and non-disclosure; Quality for route and ownership tests | Resolve open admin/recovery/resend contracts, then approve U2 |
| Bolt 3 | U3 `u3-spa-auth-sessions` | `aidlc-developer-agent` | Designer for screens/accessibility; Quality for browser journeys | Run `yarn e2e` for auth-surface changes; approve U3 |
| Bolt 4 | U4 `u4-auth-integration-proof` | `aidlc-quality-agent` | Developer for fixtures and route matrix; DevSecOps for security-flow proof | Approve integrated evidence and remaining-risk disposition |

## Coordination and ownership

- **U1 publishes** the Identity domain/application ports and typed policy outcomes that U2 consumes.
- **U2 publishes** the Identity handlers, gateway route table and HTTP/error contracts U3 consumes.
- **U3 publishes** the SPA journeys and test hooks that U4 verifies.
- **U4 owns** test and route-completeness evidence, not production behavior. A defect found by U4 is
  assigned to the owning Unit through the ordinary revision path.
- **No parallel Unit work** is allocated: the approved Unit DAG is a strict chain.
- Human authority remains with the person at every stage and Unit checkpoint; AI role labels are not
  a substitute for human approval.

# Functional Design Questions — u2-authorisation

Unit: `u2-authorisation` (U2) · Stage: construction/functional-design
Answers go after `[Answer]:`. Answering in chat is fine too — I write them back here.

## Q1. [Architect] Where the authorisation decision runs

The Contract Summary's C4 leaves this open on purpose ("Is the authorisation decision a synchronous call per request or a shared library?"), and every service's ownership check depends on the answer.

- A. A shared library — each service links the same decision code and calls it in-process; no network hop, one implementation of the rules
- B. The C4 HTTP endpoint — every service POSTs `/internal/authorize` on each request; the contract is used literally, at the cost of one internal round trip per call
- C. Hybrid — role and status decided in-process from the validated token, ownership evaluated over the C4 endpoint

[Answer]: A. A shared library — every service links the same decision code and calls it in-process

## Q2. [Architect] How ownership is proven to the decision

BR1.3 / FR15.5 require every read and write to check the owner, but the owning row usually lives in another service.

- A. The caller supplies `resourceOwnerId` after loading the row (the shape C4 already declares); the decision compares it to the caller's subject
- B. The authorisation unit asks the owning service for the owner itself, so a caller cannot lie about it
- C. The gateway stamps the owner id onto the request from its own lookup

[Answer]: A. The caller supplies `resourceOwnerId` after loading the row

## Q3. [Security] How an internal caller proves it may ask

The C4 endpoint is internal-only, but something must authenticate the caller.

- A. A service token per service plus an allowlist of which service may ask for which operation
- B. The user's own Supabase JWT, forwarded unchanged, plus the internal-service allowlist
- C. mTLS between services

[Answer]: A. A service token per service plus an allowlist of which service may ask for which operation

## Q4. [Security] A denial that cannot be audited

AC2.1.2 requires every denial to be audited, and AC7.3.2 says a security-critical action fails or is surfaced when the audit write fails.

- A. Fail closed — if the denial cannot be audited, refuse the request as well, and return it as an error
- B. Let the request through and surface the audit failure loudly (log plus metric), since a denial is already the safe outcome
- C. Refuse only on the first failure of a run, then degrade to B for a short window

[Answer]: A. Fail closed — if the denial cannot be audited, refuse the request as well, and return it as an error
